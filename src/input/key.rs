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
        KeyCode::Char('u') => {
            if app.focus == PanelFocus::Staged {
                app.unstage_selected()?;
                app.update_diff().ok();
            }
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

        // ── Diff mode ──────────────────────────────────────────────────
        KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.toggle_diff_mode();
        }

        // ── Fetch / Push / Pull ────────────────────────────────────────
        KeyCode::Char('f') => {
            app.async_status = Some("Fetching...".to_string());
            app.notify("Fetch initiated (async)".to_string(), NotificationType::Info);
        }
        KeyCode::Char('p') if !key.modifiers.contains(KeyModifiers::SHIFT) => {
            app.async_status = Some("Pushing...".to_string());
            app.notify("Push initiated (async)".to_string(), NotificationType::Info);
        }
        KeyCode::Char('P') => {
            app.async_status = Some("Pulling...".to_string());
            app.notify("Pull initiated (async)".to_string(), NotificationType::Info);
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
            // Execute selected command (placeholder for now).
            app.close_command_palette();
        }
        KeyCode::Up => {
            app.command_palette_selected = app.command_palette_selected.saturating_sub(1);
        }
        KeyCode::Down => {
            app.command_palette_selected = app.command_palette_selected.saturating_add(1);
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
            app.input_mode = InputMode::Normal;
        }
        KeyCode::Enter => {
            // TODO: perform the search with the accumulated query
            app.input_mode = InputMode::Normal;
        }
        _ => {
            // Search mode is a placeholder; we simply exit on escape/enter.
        }
    }

    Ok(())
}
