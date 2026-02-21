mod helpers;

use gittui::app::{App, InputMode, NotificationType, PanelFocus};

#[test]
fn test_panel_next_cycles() {
    let (dir, _repo) = helpers::create_test_repo();
    let mut app = App::new(dir.path().to_path_buf()).unwrap();
    app.focus = PanelFocus::Branches;
    // Tab through all panels and verify it cycles back
    let panels = [
        PanelFocus::Staged,
        PanelFocus::Changes,
        PanelFocus::Stash,
        PanelFocus::CommitMessage,
        PanelFocus::Remote,
        PanelFocus::DiffViewer,
        PanelFocus::LogGraph,
        PanelFocus::Branches, // wraps around
    ];
    for expected in panels {
        app.next_panel();
        assert_eq!(app.focus.canonical(), expected.canonical());
    }
}

#[test]
fn test_panel_prev_cycles() {
    let (dir, _repo) = helpers::create_test_repo();
    let mut app = App::new(dir.path().to_path_buf()).unwrap();
    app.focus = PanelFocus::Branches;
    app.prev_panel();
    assert_eq!(app.focus, PanelFocus::LogGraph);
}

#[test]
fn test_list_selection_next_wraps() {
    let (dir, repo) = helpers::create_test_repo();
    helpers::commit_file(&repo, "a.txt", "a", "add a");
    helpers::modify_file(&repo, "a.txt", "modified a");
    let mut app = App::new(dir.path().to_path_buf()).unwrap();
    app.focus = PanelFocus::Changes;
    // There should be at least 1 change
    assert!(app.changes_len() > 0);
    app.select_first();
    assert_eq!(app.changes_list_state.selected(), Some(0));
}

#[test]
fn test_notification_created() {
    let (dir, _repo) = helpers::create_test_repo();
    let mut app = App::new(dir.path().to_path_buf()).unwrap();
    app.notify("test".to_string(), NotificationType::Info);
    assert!(app.notification.is_some());
    assert_eq!(app.notification.as_ref().unwrap().message, "test");
}

#[test]
fn test_notification_error_flag() {
    let (dir, _repo) = helpers::create_test_repo();
    let mut app = App::new(dir.path().to_path_buf()).unwrap();
    app.notify("oops".to_string(), NotificationType::Error);
    assert!(app.notification.as_ref().unwrap().is_error);

    app.notify("ok".to_string(), NotificationType::Info);
    assert!(!app.notification.as_ref().unwrap().is_error);
}

#[test]
fn test_input_mode_transitions() {
    let (dir, _repo) = helpers::create_test_repo();
    let mut app = App::new(dir.path().to_path_buf()).unwrap();

    // Normal -> CommitEdit
    app.focus_commit_message();
    assert_eq!(app.input_mode, InputMode::EditingCommitMessage);

    // CommitEdit -> Normal
    app.leave_commit_message();
    assert_eq!(app.input_mode, InputMode::Normal);

    // Normal -> CommandPalette
    app.open_command_palette();
    assert_eq!(app.input_mode, InputMode::CommandPalette);

    // CommandPalette -> Normal
    app.close_command_palette();
    assert_eq!(app.input_mode, InputMode::Normal);
}

#[test]
fn test_app_initial_state() {
    let (dir, _repo) = helpers::create_test_repo();
    let app = App::new(dir.path().to_path_buf()).unwrap();
    assert_eq!(app.input_mode, InputMode::Normal);
    assert_eq!(app.focus, PanelFocus::Changes);
    assert!(!app.should_quit);
    assert!(!app.show_help);
    assert!(!app.show_command_palette);
    assert!(app.commit_message.is_empty());
}

#[test]
fn test_toggle_help() {
    let (dir, _repo) = helpers::create_test_repo();
    let mut app = App::new(dir.path().to_path_buf()).unwrap();
    assert!(!app.show_help);
    app.toggle_help();
    assert!(app.show_help);
    app.toggle_help();
    assert!(!app.show_help);
}

#[test]
fn test_is_clean_on_empty_repo() {
    let (dir, _repo) = helpers::create_test_repo();
    let app = App::new(dir.path().to_path_buf()).unwrap();
    assert!(app.is_clean());
}

#[test]
fn test_is_clean_with_changes() {
    let (dir, repo) = helpers::create_test_repo();
    helpers::commit_file(&repo, "a.txt", "a", "init");
    helpers::modify_file(&repo, "a.txt", "changed");
    let app = App::new(dir.path().to_path_buf()).unwrap();
    assert!(!app.is_clean());
}

#[test]
fn test_toggle_diff_mode() {
    let (dir, _repo) = helpers::create_test_repo();
    let mut app = App::new(dir.path().to_path_buf()).unwrap();
    assert_eq!(app.diff_mode, gittui::app::DiffMode::Unified);
    app.toggle_diff_mode();
    assert_eq!(app.diff_mode, gittui::app::DiffMode::SideBySide);
    app.toggle_diff_mode();
    assert_eq!(app.diff_mode, gittui::app::DiffMode::Unified);
}

#[test]
fn test_jump_to_panel() {
    let (dir, _repo) = helpers::create_test_repo();
    let mut app = App::new(dir.path().to_path_buf()).unwrap();
    app.jump_to_panel(1);
    assert_eq!(app.focus, PanelFocus::Branches);
    app.jump_to_panel(3);
    assert_eq!(app.focus, PanelFocus::Changes);
    app.jump_to_panel(6);
    assert_eq!(app.focus, PanelFocus::LogGraph);
}

#[test]
fn test_command_palette_state() {
    let (dir, _repo) = helpers::create_test_repo();
    let mut app = App::new(dir.path().to_path_buf()).unwrap();
    app.open_command_palette();
    assert!(app.show_command_palette);
    assert!(app.command_palette_input.is_empty());
    assert_eq!(app.command_palette_selected, 0);
    assert_eq!(app.input_mode, InputMode::CommandPalette);

    app.close_command_palette();
    assert!(!app.show_command_palette);
    assert_eq!(app.input_mode, InputMode::Normal);
}
