use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// Pre-computed rectangles for every panel in the application.
#[derive(Debug, Clone)]
pub struct LayoutAreas {
    pub header: Rect,
    pub left_panel: Rect,
    pub right_panel: Rect,
    pub branches: Rect,
    pub staged: Rect,
    pub changes: Rect,
    pub stash: Rect,
    pub commit: Rect,
    pub remote: Rect,
    pub diff_viewer: Rect,
    pub log_graph: Rect,
    pub statusbar: Rect,
}

impl LayoutAreas {
    /// Compute every panel area from the total terminal `area`.
    ///
    /// Layout structure:
    /// ```text
    /// ┌──────────────── header (1 line) ────────────────┐
    /// ├──── left 35% ────┬────── right 65% ─────────────┤
    /// │ Branches         │  Diff Viewer (60%)            │
    /// │ Staged           │                               │
    /// │ Changes          ├───────────────────────────────┤
    /// │ Stash            │  Log / Graph  (40%)           │
    /// │ Commit           │                               │
    /// │ Remote           │                               │
    /// ├──────────────── statusbar (1 line) ──────────────┤
    /// └─────────────────────────────────────────────────-┘
    /// ```
    pub fn compute(area: Rect) -> Self {
        // ── Outer vertical split: header | body | statusbar ───────────
        let outer = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),  // header
                Constraint::Min(6),    // body
                Constraint::Length(1), // statusbar
            ])
            .split(area);

        let header = outer[0];
        let body = outer[1];
        let statusbar = outer[2];

        // ── Body horizontal split: left panel | right panel ───────────
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(35), // left sidebar
                Constraint::Percentage(65), // right content
            ])
            .split(body);

        let left_panel = columns[0];
        let right_panel = columns[1];

        // ── Left panel: six stacked sections ──────────────────────────
        //
        // Branches and Changes get more room; Stash, Commit, and Remote
        // get a fixed minimum so they don't disappear on small terminals.
        let left_sections = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(20), // Branches
                Constraint::Percentage(18), // Staged
                Constraint::Percentage(22), // Changes
                Constraint::Percentage(12), // Stash
                Constraint::Percentage(16), // Commit
                Constraint::Percentage(12), // Remote
            ])
            .split(left_panel);

        let branches = left_sections[0];
        let staged = left_sections[1];
        let changes = left_sections[2];
        let stash = left_sections[3];
        let commit = left_sections[4];
        let remote = left_sections[5];

        // ── Right panel: diff viewer (top) / log graph (bottom) ───────
        let right_sections = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(60), // Diff viewer
                Constraint::Percentage(40), // Log / graph
            ])
            .split(right_panel);

        let diff_viewer = right_sections[0];
        let log_graph = right_sections[1];

        Self {
            header,
            left_panel,
            right_panel,
            branches,
            staged,
            changes,
            stash,
            commit,
            remote,
            diff_viewer,
            log_graph,
            statusbar,
        }
    }
}
