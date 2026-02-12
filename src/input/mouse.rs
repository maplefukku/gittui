use anyhow::Result;
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;

use crate::app::{App, PanelFocus};

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
                        select_index_in_panel(app, panel, idx);
                        // Update diff if applicable.
                        if matches!(panel, PanelFocus::Staged | PanelFocus::Changes) {
                            app.update_diff().ok();
                        }
                    }

                    // If clicking the commit message area, enter editing mode.
                    if panel == PanelFocus::CommitMessage {
                        app.focus_commit_message();
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
                        app.log_scroll = app.log_scroll.saturating_sub(3);
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
                        app.log_scroll = app.log_scroll.saturating_add(3);
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
