use std::time::Instant;

use anyhow::Result;
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;

use crate::app::{App, PanelFocus};

/// Tracks mouse click state for double-click detection.
#[derive(Debug, Clone)]
pub struct MouseState {
    pub last_click_time: Option<Instant>,
    pub last_click_col: u16,
    pub last_click_row: u16,
    pub last_click_panel: Option<PanelFocus>,
}

impl Default for MouseState {
    fn default() -> Self {
        Self {
            last_click_time: None,
            last_click_col: 0,
            last_click_row: 0,
            last_click_panel: None,
        }
    }
}

/// Layout regions for the panels.  The UI renderer should compute these and
/// pass them here so that mouse clicks can be resolved to the correct panel.
#[derive(Debug, Clone, Default)]
pub struct PanelLayout {
    pub branches: Rect,
    pub staged: Rect,
    pub changes: Rect,
    pub stash: Rect,
    pub commit_message: Rect,
    pub remote: Rect,
    pub diff_viewer: Rect,
    pub log_graph: Rect,
}

impl PanelLayout {
    /// Determine which panel (if any) contains the given (column, row).
    pub fn panel_at(&self, col: u16, row: u16) -> Option<PanelFocus> {
        if contains(self.branches, col, row) {
            Some(PanelFocus::Branches)
        } else if contains(self.staged, col, row) {
            Some(PanelFocus::Staged)
        } else if contains(self.changes, col, row) {
            Some(PanelFocus::Changes)
        } else if contains(self.stash, col, row) {
            Some(PanelFocus::Stash)
        } else if contains(self.commit_message, col, row) {
            Some(PanelFocus::CommitMessage)
        } else if contains(self.remote, col, row) {
            Some(PanelFocus::Remote)
        } else if contains(self.diff_viewer, col, row) {
            Some(PanelFocus::DiffViewer)
        } else if contains(self.log_graph, col, row) {
            Some(PanelFocus::LogGraph)
        } else {
            None
        }
    }

    /// Given a panel focus and a row inside it, compute the list item index
    /// that was clicked (accounting for the 1-row border at top).
    pub fn row_to_index(&self, panel: PanelFocus, row: u16) -> Option<usize> {
        let rect = match panel {
            PanelFocus::Branches => self.branches,
            PanelFocus::Staged => self.staged,
            PanelFocus::Changes => self.changes,
            PanelFocus::Stash => self.stash,
            PanelFocus::LogGraph => self.log_graph,
            _ => return None,
        };

        // The list content starts 1 row below the panel top (border).
        if row > rect.y && row < rect.y + rect.height.saturating_sub(1) {
            Some((row - rect.y - 1) as usize)
        } else {
            None
        }
    }
}

/// Handle a crossterm mouse event.
///
/// `layout` contains the pixel-precise panel boundaries computed during the
/// most recent render pass.  If it is `None` we ignore spatial events.
pub fn handle_mouse_event(
    app: &mut App,
    event: MouseEvent,
    layout: Option<&PanelLayout>,
    mouse_state: &mut MouseState,
) -> Result<()> {
    match event.kind {
        // ── Left click: focus panel & select item ──────────────────────
        MouseEventKind::Down(MouseButton::Left) => {
            if let Some(layout) = layout {
                let col = event.column;
                let row = event.row;

                if let Some(panel) = layout.panel_at(col, row) {
                    // Switch focus to the clicked panel.
                    app.focus = panel;

                    // If the panel is a list, select the clicked row.
                    if let Some(idx) = layout.row_to_index(panel, row) {
                        let actual_idx = if panel == PanelFocus::LogGraph {
                            idx + app.log_viewport_offset
                        } else {
                            idx
                        };
                        select_index_in_panel(app, panel, actual_idx);
                        // Update diff if applicable.
                        if matches!(panel, PanelFocus::Staged | PanelFocus::Changes) {
                            app.update_diff().ok();
                        }
                    }

                    // Check for double-click (300ms window, same position)
                    let is_double_click = if let Some(last_time) = mouse_state.last_click_time {
                        last_time.elapsed().as_millis() < 300
                            && mouse_state.last_click_col == col
                            && mouse_state.last_click_row == row
                            && mouse_state.last_click_panel == Some(panel)
                    } else {
                        false
                    };

                    // Update mouse state
                    mouse_state.last_click_time = Some(Instant::now());
                    mouse_state.last_click_col = col;
                    mouse_state.last_click_row = row;
                    mouse_state.last_click_panel = Some(panel);

                    if is_double_click {
                        // Reset to prevent triple-click being detected as another double-click
                        mouse_state.last_click_time = None;

                        match panel {
                            PanelFocus::Changes => {
                                app.stage_selected()?;
                            }
                            PanelFocus::Staged => {
                                app.unstage_selected()?;
                            }
                            PanelFocus::Branches => {
                                app.checkout_selected_branch()?;
                            }
                            _ => {}
                        }
                    }

                    // If clicking the commit message area, enter editing mode.
                    if panel == PanelFocus::CommitMessage {
                        app.focus_commit_message();
                    }
                }
            }
        }

        // ── Right click: context menu ─────────────────────────────────
        MouseEventKind::Down(MouseButton::Right) => {
            if let Some(layout) = layout {
                let col = event.column;
                let row = event.row;

                if let Some(panel) = layout.panel_at(col, row) {
                    app.focus = panel;

                    // Select the item under cursor
                    if let Some(idx) = layout.row_to_index(panel, row) {
                        let actual_idx = if panel == PanelFocus::LogGraph {
                            idx + app.log_viewport_offset
                        } else {
                            idx
                        };
                        select_index_in_panel(app, panel, actual_idx);
                    }

                    // Build context menu items based on panel
                    let items = match panel {
                        PanelFocus::Changes => vec![
                            crate::app::ContextMenuItem { label: "Stage".to_string(), shortcut: "s".to_string() },
                            crate::app::ContextMenuItem { label: "Discard".to_string(), shortcut: "d".to_string() },
                            crate::app::ContextMenuItem { label: "Stage All".to_string(), shortcut: "S".to_string() },
                            crate::app::ContextMenuItem { label: "Copy Path".to_string(), shortcut: "".to_string() },
                        ],
                        PanelFocus::Staged => vec![
                            crate::app::ContextMenuItem { label: "Unstage".to_string(), shortcut: "u".to_string() },
                            crate::app::ContextMenuItem { label: "Unstage All".to_string(), shortcut: "U".to_string() },
                            crate::app::ContextMenuItem { label: "Show Diff".to_string(), shortcut: "".to_string() },
                            crate::app::ContextMenuItem { label: "Copy Path".to_string(), shortcut: "".to_string() },
                        ],
                        PanelFocus::Branches => vec![
                            crate::app::ContextMenuItem { label: "Checkout".to_string(), shortcut: "Enter".to_string() },
                            crate::app::ContextMenuItem { label: "Create".to_string(), shortcut: "B".to_string() },
                            crate::app::ContextMenuItem { label: "Delete".to_string(), shortcut: "".to_string() },
                            crate::app::ContextMenuItem { label: "Rename".to_string(), shortcut: "".to_string() },
                        ],
                        PanelFocus::Stash => vec![
                            crate::app::ContextMenuItem { label: "Pop".to_string(), shortcut: "Z".to_string() },
                            crate::app::ContextMenuItem { label: "Apply".to_string(), shortcut: "".to_string() },
                            crate::app::ContextMenuItem { label: "Drop".to_string(), shortcut: "".to_string() },
                        ],
                        PanelFocus::LogGraph => vec![
                            crate::app::ContextMenuItem { label: "Checkout Commit".to_string(), shortcut: "".to_string() },
                            crate::app::ContextMenuItem { label: "Copy SHA".to_string(), shortcut: "".to_string() },
                        ],
                        _ => vec![],
                    };

                    if !items.is_empty() {
                        let title = match panel {
                            PanelFocus::Changes => "Changes",
                            PanelFocus::Staged => "Staged",
                            PanelFocus::Branches => "Branches",
                            PanelFocus::Stash => "Stash",
                            PanelFocus::LogGraph => "Log",
                            _ => "Menu",
                        };
                        app.context_menu = Some(crate::app::ContextMenu {
                            title: title.to_string(),
                            items,
                            selected: 0,
                            x: col,
                            y: row,
                        });
                        app.input_mode = crate::app::InputMode::ContextMenu;
                    }
                }
            }
        }

        // ── Scroll up ─────────────────────────────────────────────────
        MouseEventKind::ScrollUp => {
            if let Some(layout) = layout {
                let panel = layout.panel_at(event.column, event.row);
                match panel {
                    Some(PanelFocus::DiffViewer) => {
                        app.diff_scroll = app.diff_scroll.saturating_sub(3);
                    }
                    Some(PanelFocus::LogGraph) => {
                        let prev_focus = app.focus;
                        app.focus = PanelFocus::LogGraph;
                        for _ in 0..3 {
                            app.select_prev();
                        }
                        app.focus = prev_focus;
                    }
                    Some(p) => {
                        // Scroll lists by selecting previous items.
                        let prev_focus = app.focus;
                        app.focus = p;
                        for _ in 0..3 {
                            app.select_prev();
                        }
                        app.focus = prev_focus;
                    }
                    None => {}
                }
            }
        }

        // ── Scroll down ───────────────────────────────────────────────
        MouseEventKind::ScrollDown => {
            if let Some(layout) = layout {
                let panel = layout.panel_at(event.column, event.row);
                match panel {
                    Some(PanelFocus::DiffViewer) => {
                        app.diff_scroll = app.diff_scroll.saturating_add(3);
                    }
                    Some(PanelFocus::LogGraph) => {
                        let prev_focus = app.focus;
                        app.focus = PanelFocus::LogGraph;
                        for _ in 0..3 {
                            app.select_next();
                        }
                        app.focus = prev_focus;
                    }
                    Some(p) => {
                        let prev_focus = app.focus;
                        app.focus = p;
                        for _ in 0..3 {
                            app.select_next();
                        }
                        app.focus = prev_focus;
                    }
                    None => {}
                }
            }
        }

        // Ignore all other mouse events (drag, move, etc.).
        _ => {}
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Check whether (col, row) is inside `rect`.
fn contains(rect: Rect, col: u16, row: u16) -> bool {
    col >= rect.x
        && col < rect.x + rect.width
        && row >= rect.y
        && row < rect.y + rect.height
}

/// Select a specific index in the list for the given panel.
fn select_index_in_panel(app: &mut App, panel: PanelFocus, idx: usize) {
    match panel {
        PanelFocus::Branches => {
            if idx < app.branches.len() {
                app.branch_list_state.select(Some(idx));
            }
        }
        PanelFocus::Staged => {
            if idx < app.staged.len() {
                app.staged_list_state.select(Some(idx));
            }
        }
        PanelFocus::Changes => {
            if idx < app.changes_len() {
                app.changes_list_state.select(Some(idx));
            }
        }
        PanelFocus::Stash => {
            if idx < app.stashes.len() {
                app.stash_list_state.select(Some(idx));
            }
        }
        PanelFocus::LogGraph => {
            if idx < app.log.len() {
                app.log_list_state.select(Some(idx));
            }
        }
        _ => {}
    }
}
