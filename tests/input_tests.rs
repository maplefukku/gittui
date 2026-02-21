mod helpers;

use crossterm::event::{
    KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::layout::Rect;

use gittui::app::{App, InputMode, PanelFocus};
use gittui::input::key::handle_key_event;
use gittui::input::mouse::{handle_mouse_event, MouseState, PanelLayout};

/// Create an App with a test repo that has at least one commit and one
/// modified file so that both Staged and Changes panels have content.
fn setup_app_with_changes() -> (tempfile::TempDir, App) {
    let (dir, repo) = helpers::create_test_repo();
    helpers::commit_file(&repo, "file.txt", "initial", "init");
    helpers::modify_file(&repo, "file.txt", "changed");
    let app = App::new(dir.path().to_path_buf()).unwrap();
    (dir, app)
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn key_ctrl(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::CONTROL)
}

fn make_layout() -> PanelLayout {
    PanelLayout {
        branches: Rect::new(0, 0, 30, 10),
        staged: Rect::new(0, 10, 30, 10),
        changes: Rect::new(0, 20, 30, 10),
        stash: Rect::new(0, 30, 30, 10),
        commit_message: Rect::new(30, 0, 30, 5),
        remote: Rect::new(30, 5, 30, 5),
        diff_viewer: Rect::new(30, 10, 30, 20),
        log_graph: Rect::new(30, 30, 30, 10),
    }
}

// ---------------------------------------------------------------------------
// Keyboard tests
// ---------------------------------------------------------------------------

#[test]
fn test_quit_key() {
    let (_dir, mut app) = setup_app_with_changes();
    assert!(!app.should_quit);
    handle_key_event(&mut app, key(KeyCode::Char('q'))).unwrap();
    assert!(app.should_quit);
}

#[test]
fn test_ctrl_c_quits() {
    let (_dir, mut app) = setup_app_with_changes();
    handle_key_event(&mut app, key_ctrl(KeyCode::Char('c'))).unwrap();
    assert!(app.should_quit);
}

#[test]
fn test_j_k_navigation() {
    let (_dir, mut app) = setup_app_with_changes();
    app.focus = PanelFocus::Changes;
    // Ensure there's at least one item
    assert!(app.changes_len() > 0);
    // j moves selection down
    handle_key_event(&mut app, key(KeyCode::Char('j'))).unwrap();
    let sel = app.changes_list_state.selected();
    assert!(sel.is_some());
}

#[test]
fn test_tab_switches_panel() {
    let (_dir, mut app) = setup_app_with_changes();
    app.focus = PanelFocus::Branches;
    handle_key_event(&mut app, key(KeyCode::Tab)).unwrap();
    assert_eq!(app.focus.canonical(), PanelFocus::Staged.canonical());
}

#[test]
fn test_command_palette_opens_and_closes() {
    let (_dir, mut app) = setup_app_with_changes();
    // Open command palette with Ctrl+P
    handle_key_event(&mut app, key_ctrl(KeyCode::Char('p'))).unwrap();
    assert_eq!(app.input_mode, InputMode::CommandPalette);
    assert!(app.show_command_palette);
    // Close with Esc
    handle_key_event(&mut app, key(KeyCode::Esc)).unwrap();
    assert_eq!(app.input_mode, InputMode::Normal);
    assert!(!app.show_command_palette);
}

#[test]
fn test_command_palette_typing_filters() {
    let (_dir, mut app) = setup_app_with_changes();
    handle_key_event(&mut app, key_ctrl(KeyCode::Char('p'))).unwrap();
    // Type a character
    handle_key_event(&mut app, key(KeyCode::Char('s'))).unwrap();
    assert_eq!(app.command_palette_input, "s");
    // Selection resets to 0 on input
    assert_eq!(app.command_palette_selected, 0);
}

#[test]
fn test_help_toggle() {
    let (_dir, mut app) = setup_app_with_changes();
    assert!(!app.show_help);
    handle_key_event(&mut app, key(KeyCode::Char('?'))).unwrap();
    assert!(app.show_help);
    // Any key dismisses help
    handle_key_event(&mut app, key(KeyCode::Char('x'))).unwrap();
    assert!(!app.show_help);
}

// ---------------------------------------------------------------------------
// Mouse tests
// ---------------------------------------------------------------------------

#[test]
fn test_left_click_focuses_panel() {
    let (_dir, mut app) = setup_app_with_changes();
    let layout = make_layout();
    let mut mouse_state = MouseState::default();

    // Click inside the changes panel area (row 21 is inside changes: y=20, h=10)
    let event = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 5,
        row: 21,
        modifiers: KeyModifiers::NONE,
    };
    handle_mouse_event(&mut app, event, Some(&layout), &mut mouse_state).unwrap();
    assert_eq!(app.focus, PanelFocus::Changes);
}

#[test]
fn test_left_click_focuses_branches() {
    let (_dir, mut app) = setup_app_with_changes();
    let layout = make_layout();
    let mut mouse_state = MouseState::default();

    // Click inside branches panel (row 1 is inside branches: y=0, h=10)
    let event = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 5,
        row: 1,
        modifiers: KeyModifiers::NONE,
    };
    handle_mouse_event(&mut app, event, Some(&layout), &mut mouse_state).unwrap();
    assert_eq!(app.focus, PanelFocus::Branches);
}

#[test]
fn test_right_click_opens_context_menu() {
    let (_dir, mut app) = setup_app_with_changes();
    let layout = make_layout();
    let mut mouse_state = MouseState::default();

    // Right-click inside the changes panel
    let event = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Right),
        column: 5,
        row: 21,
        modifiers: KeyModifiers::NONE,
    };
    handle_mouse_event(&mut app, event, Some(&layout), &mut mouse_state).unwrap();
    assert!(app.context_menu.is_some());
    assert_eq!(app.input_mode, InputMode::ContextMenu);

    let menu = app.context_menu.as_ref().unwrap();
    assert_eq!(menu.title, "Changes");
    // Should have Stage, Discard, Stage All, Copy Path
    assert!(menu.items.len() >= 3);
    assert_eq!(menu.items[0].label, "Stage");
}

#[test]
fn test_right_click_staged_context_menu() {
    let (_dir, mut app) = setup_app_with_changes();
    let layout = make_layout();
    let mut mouse_state = MouseState::default();

    // Right-click inside the staged panel (row 11 is inside staged: y=10, h=10)
    let event = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Right),
        column: 5,
        row: 11,
        modifiers: KeyModifiers::NONE,
    };
    handle_mouse_event(&mut app, event, Some(&layout), &mut mouse_state).unwrap();
    assert!(app.context_menu.is_some());
    let menu = app.context_menu.as_ref().unwrap();
    assert_eq!(menu.title, "Staged");
    assert_eq!(menu.items[0].label, "Unstage");
}

#[test]
fn test_context_menu_keyboard_navigation() {
    let (_dir, mut app) = setup_app_with_changes();
    let layout = make_layout();
    let mut mouse_state = MouseState::default();

    // Open context menu on changes panel
    let event = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Right),
        column: 5,
        row: 21,
        modifiers: KeyModifiers::NONE,
    };
    handle_mouse_event(&mut app, event, Some(&layout), &mut mouse_state).unwrap();
    assert_eq!(app.input_mode, InputMode::ContextMenu);
    assert_eq!(app.context_menu.as_ref().unwrap().selected, 0);

    // Press down to move selection
    handle_key_event(&mut app, key(KeyCode::Down)).unwrap();
    assert_eq!(app.context_menu.as_ref().unwrap().selected, 1);

    // Press up to move back
    handle_key_event(&mut app, key(KeyCode::Up)).unwrap();
    assert_eq!(app.context_menu.as_ref().unwrap().selected, 0);

    // Press Esc to close
    handle_key_event(&mut app, key(KeyCode::Esc)).unwrap();
    assert!(app.context_menu.is_none());
    assert_eq!(app.input_mode, InputMode::Normal);
}

#[test]
fn test_scroll_up_selects_prev_in_log() {
    let (_dir, mut app) = setup_app_with_changes();
    let layout = make_layout();
    let mut mouse_state = MouseState::default();

    // Need at least some log entries
    if app.log.is_empty() {
        return;
    }

    // Select a non-zero index first
    app.focus = PanelFocus::LogGraph;
    app.log_list_state.select(Some(3.min(app.log.len().saturating_sub(1))));

    let before = app.log_list_state.selected().unwrap_or(0);

    // Scroll up inside log graph panel (row 31 is inside log_graph: y=30, h=10)
    let event = MouseEvent {
        kind: MouseEventKind::ScrollUp,
        column: 35,
        row: 31,
        modifiers: KeyModifiers::NONE,
    };
    handle_mouse_event(&mut app, event, Some(&layout), &mut mouse_state).unwrap();

    let after = app.log_list_state.selected().unwrap_or(0);
    // Selection should have moved (or stayed at 0 if already there)
    assert!(after <= before);
}

#[test]
fn test_scroll_down_selects_next_in_log() {
    let (_dir, mut app) = setup_app_with_changes();
    let layout = make_layout();
    let mut mouse_state = MouseState::default();

    if app.log.is_empty() {
        return;
    }

    app.focus = PanelFocus::LogGraph;
    app.log_list_state.select(Some(0));

    // Scroll down inside log graph panel
    let event = MouseEvent {
        kind: MouseEventKind::ScrollDown,
        column: 35,
        row: 31,
        modifiers: KeyModifiers::NONE,
    };
    handle_mouse_event(&mut app, event, Some(&layout), &mut mouse_state).unwrap();

    let after = app.log_list_state.selected().unwrap_or(0);
    // Selection should have advanced (wrapping is ok for small lists)
    assert!(after > 0 || app.log.len() <= 1);
}

#[test]
fn test_double_click_detection_requires_same_position() {
    let (_dir, mut app) = setup_app_with_changes();
    let layout = make_layout();
    let mut mouse_state = MouseState::default();

    // First click at (5, 21) in changes
    let event1 = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 5,
        row: 21,
        modifiers: KeyModifiers::NONE,
    };
    handle_mouse_event(&mut app, event1, Some(&layout), &mut mouse_state).unwrap();
    assert!(mouse_state.last_click_time.is_some());

    // Second click at a different position -- should NOT be a double click
    let event2 = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 10,
        row: 25,
        modifiers: KeyModifiers::NONE,
    };
    handle_mouse_event(&mut app, event2, Some(&layout), &mut mouse_state).unwrap();
    // MouseState should still have a click time (not reset like double-click does)
    assert!(mouse_state.last_click_time.is_some());
}

#[test]
fn test_mouse_state_default() {
    let state = MouseState::default();
    assert!(state.last_click_time.is_none());
    assert_eq!(state.last_click_col, 0);
    assert_eq!(state.last_click_row, 0);
    assert!(state.last_click_panel.is_none());
}

#[test]
fn test_log_viewport_offset_on_click() {
    let (_dir, mut app) = setup_app_with_changes();
    let layout = make_layout();
    let mut mouse_state = MouseState::default();

    if app.log.len() < 5 {
        return;
    }

    // Set viewport offset to 3
    app.log_viewport_offset = 3;

    // Click row 31 (first content row in log_graph: y=30 + 1 for border = row 31)
    // This should select index 0 + viewport_offset = 3
    let event = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 35,
        row: 31,
        modifiers: KeyModifiers::NONE,
    };
    handle_mouse_event(&mut app, event, Some(&layout), &mut mouse_state).unwrap();
    assert_eq!(app.log_list_state.selected(), Some(3));
}
