use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::{App, InputMode, NotificationType, PanelFocus};
use crate::input::text_editor;

/// Top-level keyboard event dispatcher.
///
/// Routes the event to the correct handler based on the current `InputMode`.
pub fn handle_key_event(app: &mut App, key: KeyEvent) -> Result<()> {
    // Always allow Ctrl+C to quit.
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        app.should_quit = true;
        return Ok(());
    }

    match app.input_mode {
        InputMode::Normal => handle_normal_mode(app, key),
        InputMode::Editing | InputMode::EditingCommitMessage => handle_commit_edit_mode(app, key),
        InputMode::CommandPalette => handle_command_palette_mode(app, key),
        InputMode::DialogInput => handle_dialog_mode(app, key),
        InputMode::SearchInput => handle_search_mode(app, key),
        InputMode::ContextMenu => handle_context_menu_mode(app, key),
    }
}

// ---------------------------------------------------------------------------
// Normal mode
// ---------------------------------------------------------------------------

fn handle_normal_mode(app: &mut App, key: KeyEvent) -> Result<()> {
    // If help overlay is active, any key dismisses it.
    if app.show_help {
        app.show_help = false;
        return Ok(());
    }

    match key.code {
        // ── Quit ───────────────────────────────────────────────────────
        KeyCode::Char('q') => {
            app.should_quit = true;
        }

        // ── Panel navigation ───────────────────────────────────────────
        KeyCode::Tab => {
            app.next_panel();
            // Update diff when switching to staged/changes panels.
            if matches!(app.focus, PanelFocus::Staged | PanelFocus::Changes) {
                app.update_diff().ok();
            }
        }
        KeyCode::BackTab => {
            app.prev_panel();
            if matches!(app.focus, PanelFocus::Staged | PanelFocus::Changes) {
                app.update_diff().ok();
            }
        }

        // Number keys jump to panels.
        KeyCode::Char(c @ '1'..='6') => {
            let n = c as u8 - b'0';
            app.jump_to_panel(n);
            if matches!(app.focus, PanelFocus::Staged | PanelFocus::Changes) {
                app.update_diff().ok();
            }
        }

        // h/l for left/right pane switch (same as Shift-Tab / Tab).
        KeyCode::Char('h') => {
            app.prev_panel();
            if matches!(app.focus, PanelFocus::Staged | PanelFocus::Changes) {
                app.update_diff().ok();
            }
        }
        KeyCode::Char('l') => {
            app.next_panel();
            if matches!(app.focus, PanelFocus::Staged | PanelFocus::Changes) {
                app.update_diff().ok();
            }
        }

        // ── List navigation ────────────────────────────────────────────
        KeyCode::Char('j') | KeyCode::Down => {
            app.select_next();
            if matches!(app.focus, PanelFocus::Staged | PanelFocus::Changes) {
                app.update_diff().ok();
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.select_prev();
            if matches!(app.focus, PanelFocus::Staged | PanelFocus::Changes) {
                app.update_diff().ok();
            }
        }
        KeyCode::PageDown => {
            if app.focus == PanelFocus::DiffViewer {
                app.diff_scroll = app.diff_scroll.saturating_add(20);
            }
        }
        KeyCode::PageUp => {
            if app.focus == PanelFocus::DiffViewer {
                app.diff_scroll = app.diff_scroll.saturating_sub(20);
            }
        }
        KeyCode::Char('g') => {
            app.select_first();
        }
        KeyCode::Char('G') => {
            app.select_last();
        }

        // ── Stage / unstage ────────────────────────────────────────────
        KeyCode::Char('s') => {
            if app.focus == PanelFocus::Changes {
                app.stage_selected()?;
                app.update_diff().ok();
            }
        }
        KeyCode::Char('u') if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            if app.focus == PanelFocus::Staged {
                app.unstage_selected()?;
                app.update_diff().ok();
            }
        }
        KeyCode::Char('S') => {
            app.stage_all()?;
        }
        KeyCode::Char('U') => {
            app.unstage_all()?;
        }

        // ── Commit ─────────────────────────────────────────────────────
        KeyCode::Char('c') => {
            app.focus_commit_message();
        }
        KeyCode::Char('a') => {
            app.toggle_amend();
        }

        // Ctrl+Enter to commit (crossterm sends Char('\n') with no modifiers
        // for Enter, and with CONTROL for Ctrl+Enter on some terminals).
        KeyCode::Enter => {
            match app.focus {
                PanelFocus::Branches => {
                    app.checkout_selected_branch()?;
                }
                PanelFocus::CommitMessage => {
                    // Enter in normal mode on commit panel -> focus it for editing
                    app.focus_commit_message();
                }
                _ => {}
            }
        }

        // ── Discard ────────────────────────────────────────────────────
        KeyCode::Char('d') if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.discard_selected()?;
        }

        // ── Diff mode / half-page scroll ────────────────────────────────
        KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if app.focus == PanelFocus::DiffViewer {
                app.diff_scroll = app.diff_scroll.saturating_add(10);
            } else {
                app.toggle_diff_mode();
            }
        }
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if app.focus == PanelFocus::DiffViewer {
                app.diff_scroll = app.diff_scroll.saturating_sub(10);
            }
        }

        // ── Fetch / Push / Pull ────────────────────────────────────────
        KeyCode::Char('f') => {
            if app.async_op.is_none() {
                app.async_op = Some(crate::app::AsyncOp::Fetching);
                app.async_status = Some("Fetching...".to_string());

                if let Some(tx) = app.async_tx.clone() {
                    let repo_path = app.repo_path.clone();
                    std::thread::spawn(move || {
                        let result = (|| -> anyhow::Result<String> {
                            let repo = crate::git::repo::open_repo(&repo_path)?;
                            crate::git::remote::fetch(&repo, "origin")?;
                            Ok("Fetch complete".to_string())
                        })();
                        let _ = tx.send(crate::event::AsyncResult::FetchComplete(result));
                    });
                }
            }
        }
        KeyCode::Char('p') if !key.modifiers.contains(KeyModifiers::SHIFT) && !key.modifiers.contains(KeyModifiers::CONTROL) => {
            if app.async_op.is_none() {
                app.async_op = Some(crate::app::AsyncOp::Pushing);
                app.async_status = Some("Pushing...".to_string());

                if let Some(tx) = app.async_tx.clone() {
                    let repo_path = app.repo_path.clone();
                    let branch = app.head.branch.clone().unwrap_or_else(|| "main".to_string());
                    std::thread::spawn(move || {
                        let result = (|| -> anyhow::Result<String> {
                            let repo = crate::git::repo::open_repo(&repo_path)?;
                            crate::git::remote::push(&repo, "origin", &branch)?;
                            Ok("Push complete".to_string())
                        })();
                        let _ = tx.send(crate::event::AsyncResult::PushComplete(result));
                    });
                }
            }
        }
        KeyCode::Char('P') => {
            if app.async_op.is_none() {
                app.async_op = Some(crate::app::AsyncOp::Pulling);
                app.async_status = Some("Pulling...".to_string());

                if let Some(tx) = app.async_tx.clone() {
                    let repo_path = app.repo_path.clone();
                    let branch = app.head.branch.clone().unwrap_or_else(|| "main".to_string());
                    std::thread::spawn(move || {
                        let result = (|| -> anyhow::Result<String> {
                            let repo = crate::git::repo::open_repo(&repo_path)?;
                            crate::git::remote::pull(&repo, "origin", &branch)?;
                            Ok("Pull complete".to_string())
                        })();
                        let _ = tx.send(crate::event::AsyncResult::PullComplete(result));
                    });
                }
            }
        }

        // ── Branches ───────────────────────────────────────────────────
        KeyCode::Char('b') => {
            app.focus = PanelFocus::Branches;
        }
        KeyCode::Char('B') => {
            app.create_branch()?;
        }

        // ── Stash ──────────────────────────────────────────────────────
        KeyCode::Char('z') => {
            app.stash_save()?;
        }
        KeyCode::Char('Z') => {
            app.stash_pop()?;
        }

        // ── Refresh ────────────────────────────────────────────────────
        KeyCode::Char('r') => {
            app.refresh()?;
            app.notify("Refreshed".to_string(), NotificationType::Info);
        }

        // ── Search ─────────────────────────────────────────────────────
        KeyCode::Char('/') => {
            app.input_mode = InputMode::SearchInput;
        }

        // ── Help ───────────────────────────────────────────────────────
        KeyCode::Char('?') => {
            app.toggle_help();
        }

        // ── Command palette ────────────────────────────────────────────
        KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.open_command_palette();
        }

        _ => {}
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Commit message editing mode
// ---------------------------------------------------------------------------

fn handle_commit_edit_mode(app: &mut App, key: KeyEvent) -> Result<()> {
    match key.code {
        // Escape leaves commit editing.
        KeyCode::Esc => {
            app.leave_commit_message();
        }

        // Ctrl+Enter -> perform commit.
        KeyCode::Enter if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.do_commit()?;
            app.leave_commit_message();
        }

        // Tab in commit message -> leave editing and go to next panel.
        KeyCode::Tab => {
            app.leave_commit_message();
            app.next_panel();
        }
        KeyCode::BackTab => {
            app.leave_commit_message();
            app.prev_panel();
        }

        // Ctrl+G -> trigger AI commit message generation
        KeyCode::Char('g') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if !app.ai_generating {
                app.ai_requested = true;
                app.notify("Generating AI commit message...".to_string(), NotificationType::Info);
            }
        }

        // All other keys go to the mini text editor.
        _ => {
            text_editor::handle_text_input(
                &mut app.commit_message,
                &mut app.commit_cursor,
                key,
            );
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Command palette mode
// ---------------------------------------------------------------------------

fn handle_command_palette_mode(app: &mut App, key: KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Esc => {
            app.close_command_palette();
        }
        KeyCode::Enter => {
            // Get the selected command name from the filtered list
            let query = app.command_palette_input.clone();
            let filtered: Vec<&crate::app::PaletteCommand> = if query.is_empty() {
                app.palette_commands.iter().collect()
            } else {
                use fuzzy_matcher::FuzzyMatcher;
                use fuzzy_matcher::skim::SkimMatcherV2;
                let matcher = SkimMatcherV2::default();
                app.palette_commands
                    .iter()
                    .filter(|cmd| matcher.fuzzy_match(&cmd.name, &query).is_some())
                    .collect()
            };

            let selected_idx = app
                .command_palette_selected
                .min(filtered.len().saturating_sub(1));
            let command_name = filtered.get(selected_idx).map(|cmd| cmd.name.clone());

            app.close_command_palette();

            if let Some(name) = command_name {
                execute_palette_command(app, &name)?;
            }
        }
        KeyCode::Up => {
            app.command_palette_selected = app.command_palette_selected.saturating_sub(1);
        }
        KeyCode::Down => {
            // Clamp to filtered list length
            let query = &app.command_palette_input;
            let filtered_len = if query.is_empty() {
                app.palette_commands.len()
            } else {
                use fuzzy_matcher::FuzzyMatcher;
                use fuzzy_matcher::skim::SkimMatcherV2;
                let matcher = SkimMatcherV2::default();
                app.palette_commands
                    .iter()
                    .filter(|cmd| matcher.fuzzy_match(&cmd.name, query).is_some())
                    .count()
            };
            app.command_palette_selected =
                (app.command_palette_selected + 1).min(filtered_len.saturating_sub(1));
        }
        KeyCode::Backspace => {
            app.command_palette_input.pop();
            app.command_palette_selected = 0;
        }
        KeyCode::Char(c) => {
            app.command_palette_input.push(c);
            app.command_palette_selected = 0;
        }
        _ => {}
    }

    Ok(())
}

/// Execute a command palette command by name.
fn execute_palette_command(app: &mut App, name: &str) -> Result<()> {
    match name {
        "Stage File" => {
            app.stage_selected()?;
        }
        "Unstage File" => {
            app.unstage_selected()?;
        }
        "Commit" => {
            app.do_commit()?;
        }
        "Amend Commit" => {
            app.toggle_amend();
        }
        "Discard Changes" => {
            app.discard_selected()?;
        }
        "Fetch" => {
            app.async_status = Some("Fetching...".to_string());
            app.notify("Fetch initiated".to_string(), NotificationType::Info);
        }
        "Push" => {
            app.async_status = Some("Pushing...".to_string());
            app.notify("Push initiated".to_string(), NotificationType::Info);
        }
        "Pull" => {
            app.async_status = Some("Pulling...".to_string());
            app.notify("Pull initiated".to_string(), NotificationType::Info);
        }
        "Create Branch" => {
            app.create_branch()?;
        }
        "Stash Save" => {
            app.stash_save()?;
        }
        "Stash Pop" => {
            app.stash_pop()?;
        }
        "Refresh" => {
            app.refresh()?;
            app.notify("Refreshed".to_string(), NotificationType::Info);
        }
        "Toggle Diff Mode" => {
            app.toggle_diff_mode();
        }
        "Help" => {
            app.toggle_help();
        }
        "Quit" => {
            app.should_quit = true;
        }
        _ => {}
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Dialog input mode
// ---------------------------------------------------------------------------

fn handle_dialog_mode(app: &mut App, key: KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Esc => {
            app.cancel_dialog();
        }
        KeyCode::Enter => {
            app.confirm_dialog()?;
        }
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            // For Confirm dialogs, 'y' acts as confirm.
            if let Some(ref dialog) = app.dialog {
                if matches!(dialog.dialog_type, crate::app::DialogType::Confirm { .. }) {
                    app.confirm_dialog()?;
                    return Ok(());
                }
            }
            // For input dialogs, type the character.
            if let Some(ref mut dialog) = app.dialog {
                dialog.input.push('y');
            }
        }
        KeyCode::Char('n') | KeyCode::Char('N') => {
            // For Confirm dialogs, 'n' acts as cancel.
            if let Some(ref dialog) = app.dialog {
                if matches!(dialog.dialog_type, crate::app::DialogType::Confirm { .. }) {
                    app.cancel_dialog();
                    return Ok(());
                }
            }
            if let Some(ref mut dialog) = app.dialog {
                dialog.input.push('n');
            }
        }
        KeyCode::Backspace => {
            if let Some(ref mut dialog) = app.dialog {
                dialog.input.pop();
            }
        }
        KeyCode::Char(c) => {
            if let Some(ref mut dialog) = app.dialog {
                dialog.input.push(c);
            }
        }
        KeyCode::Up => {
            if let Some(ref mut dialog) = app.dialog {
                dialog.selected = dialog.selected.saturating_sub(1);
            }
        }
        KeyCode::Down => {
            if let Some(ref mut dialog) = app.dialog {
                dialog.selected = dialog.selected.saturating_add(1);
            }
        }
        _ => {}
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Search mode
// ---------------------------------------------------------------------------

fn handle_search_mode(app: &mut App, key: KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Esc => {
            app.search_query.clear();
            app.search_results.clear();
            app.input_mode = InputMode::Normal;
        }
        KeyCode::Enter => {
            // Keep results and exit search mode
            app.input_mode = InputMode::Normal;
        }
        KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.search_next();
        }
        KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.search_prev();
        }
        KeyCode::Backspace => {
            app.search_query.pop();
            app.perform_search();
        }
        KeyCode::Char(c) => {
            app.search_query.push(c);
            app.perform_search();
        }
        _ => {}
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Context menu mode
// ---------------------------------------------------------------------------

fn handle_context_menu_mode(app: &mut App, key: KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Esc => {
            app.context_menu = None;
            app.input_mode = InputMode::Normal;
        }
        KeyCode::Enter => {
            app.execute_context_menu_action()?;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if let Some(ref mut menu) = app.context_menu {
                menu.selected = menu.selected.saturating_sub(1);
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if let Some(ref mut menu) = app.context_menu {
                let max = menu.items.len().saturating_sub(1);
                menu.selected = (menu.selected + 1).min(max);
            }
        }
        _ => {}
    }
    Ok(())
}
