use std::path::PathBuf;
use std::time::Instant;

use anyhow::{Context, Result};
use ratatui::widgets::ListState;

use crate::git::{
    self, BranchInfo, CommitInfo, FileDiff, FileStatus, HeadInfo, RemoteInfo, StashInfo,
};

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

/// Which panel currently has focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelFocus {
    Branches,
    Staged,
    Changes,
    Stash,
    CommitMessage,
    /// Alias used by commit UI: `PanelFocus::Commit` == `PanelFocus::CommitMessage`.
    Commit,
    Remote,
    DiffViewer,
    LogGraph,
}

impl PanelFocus {
    /// All panels in tab-order.
    pub const ORDER: [PanelFocus; 8] = [
        PanelFocus::Branches,
        PanelFocus::Staged,
        PanelFocus::Changes,
        PanelFocus::Stash,
        PanelFocus::CommitMessage,
        PanelFocus::Remote,
        PanelFocus::DiffViewer,
        PanelFocus::LogGraph,
    ];

    /// Index of this panel in the tab order.
    fn index(self) -> usize {
        let canonical = self.canonical();
        Self::ORDER
            .iter()
            .position(|&p| p == canonical)
            .unwrap_or(0)
    }

    /// Normalize aliases.
    fn canonical(self) -> Self {
        match self {
            PanelFocus::Commit => PanelFocus::CommitMessage,
            other => other,
        }
    }

    /// Next panel in the cycle.
    pub fn next(self) -> Self {
        let idx = (self.index() + 1) % Self::ORDER.len();
        Self::ORDER[idx]
    }

    /// Previous panel in the cycle.
    pub fn prev(self) -> Self {
        let idx = (self.index() + Self::ORDER.len() - 1) % Self::ORDER.len();
        Self::ORDER[idx]
    }

    /// Convert a 1-based panel number to a PanelFocus (keys 1-6 map to the
    /// first six panels).
    pub fn from_number(n: u8) -> Option<Self> {
        match n {
            1 => Some(PanelFocus::Branches),
            2 => Some(PanelFocus::Staged),
            3 => Some(PanelFocus::Changes),
            4 => Some(PanelFocus::Stash),
            5 => Some(PanelFocus::CommitMessage),
            6 => Some(PanelFocus::LogGraph),
            _ => None,
        }
    }
}

/// The current input mode of the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    EditingCommitMessage,
    /// Alias: `InputMode::Editing` == `InputMode::EditingCommitMessage`.
    Editing,
    CommandPalette,
    DialogInput,
    SearchInput,
}

impl InputMode {
    /// Normalize aliases for comparison.
    pub fn canonical(self) -> Self {
        match self {
            InputMode::Editing => InputMode::EditingCommitMessage,
            other => other,
        }
    }
}

/// How diffs are displayed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffMode {
    Unified,
    SideBySide,
}

/// Notification severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationType {
    Info,
    Success,
    Error,
    Warning,
}

/// Currently running async operation (fetch / push / pull).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AsyncOp {
    Fetching,
    Pulling,
    Pushing,
}

// ---------------------------------------------------------------------------
// Command palette
// ---------------------------------------------------------------------------

/// A single entry in the command palette.
#[derive(Debug, Clone)]
pub struct PaletteCommand {
    pub name: String,
    pub shortcut: String,
}

impl PaletteCommand {
    pub fn new(name: &str, shortcut: &str) -> Self {
        Self {
            name: name.to_string(),
            shortcut: shortcut.to_string(),
        }
    }
}

/// Build the default palette command list.
fn default_palette_commands() -> Vec<PaletteCommand> {
    vec![
        PaletteCommand::new("Stage File", "s"),
        PaletteCommand::new("Unstage File", "u"),
        PaletteCommand::new("Commit", "Ctrl+Enter"),
        PaletteCommand::new("Amend Commit", "a"),
        PaletteCommand::new("Discard Changes", "d"),
        PaletteCommand::new("Fetch", "f"),
        PaletteCommand::new("Push", "p"),
        PaletteCommand::new("Pull", "P"),
        PaletteCommand::new("Create Branch", "B"),
        PaletteCommand::new("Stash Save", "z"),
        PaletteCommand::new("Stash Pop", "Z"),
        PaletteCommand::new("Refresh", "r"),
        PaletteCommand::new("Toggle Diff Mode", "Ctrl+D"),
        PaletteCommand::new("Help", "?"),
        PaletteCommand::new("Quit", "q"),
    ]
}

// ---------------------------------------------------------------------------
// Context menu
// ---------------------------------------------------------------------------

/// An item in a context menu.
#[derive(Debug, Clone)]
pub struct ContextMenuItem {
    pub label: String,
    pub shortcut: String,
}

/// A floating context menu.
#[derive(Debug, Clone)]
pub struct ContextMenu {
    pub title: String,
    pub items: Vec<ContextMenuItem>,
    pub selected: usize,
    pub x: u16,
    pub y: u16,
}

// ---------------------------------------------------------------------------
// Notification (structured)
// ---------------------------------------------------------------------------

/// A notification message displayed in the status bar.
#[derive(Debug, Clone)]
pub struct Notification {
    pub message: String,
    pub is_error: bool,
    pub when: Instant,
}

// ---------------------------------------------------------------------------
// Dialog types
// ---------------------------------------------------------------------------

/// A modal dialog shown on top of the normal UI.
#[derive(Debug)]
pub struct Dialog {
    pub title: String,
    pub message: String,
    pub dialog_type: DialogType,
    pub input: String,
    pub selected: usize,
}

#[derive(Debug)]
pub enum DialogType {
    Confirm { on_confirm: DialogAction },
    Input { on_submit: DialogAction },
    Select {
        options: Vec<String>,
        on_select: DialogAction,
    },
}

#[derive(Debug, Clone)]
pub enum DialogAction {
    DeleteBranch(String),
    DiscardChanges(String),
    ForcePush,
    CreateBranch,
    RenameBranch(String),
    StashSave,
    StashDrop(usize),
    CheckoutBranch(String),
}

// ---------------------------------------------------------------------------
// App
// ---------------------------------------------------------------------------

/// Central application state.
pub struct App {
    pub repo_path: PathBuf,

    // ── Git state ──────────────────────────────────────────────────────
    pub head: HeadInfo,
    pub branches: Vec<BranchInfo>,
    pub staged: Vec<FileStatus>,
    pub unstaged: Vec<FileStatus>,
    pub untracked: Vec<FileStatus>,
    pub conflicts: Vec<FileStatus>,
    pub stashes: Vec<StashInfo>,
    pub remotes: Vec<RemoteInfo>,
    pub log: Vec<CommitInfo>,
    pub ahead: usize,
    pub behind: usize,

    // ── UI / list state ────────────────────────────────────────────────
    pub focus: PanelFocus,
    pub branch_list_state: ListState,
    pub staged_list_state: ListState,
    pub changes_list_state: ListState,
    pub stash_list_state: ListState,
    pub log_list_state: ListState,
    pub diff_scroll: u16,
    pub log_scroll: u16,

    // ── Diff ───────────────────────────────────────────────────────────
    pub current_diff: Option<FileDiff>,
    pub diff_mode: DiffMode,

    // ── Commit ─────────────────────────────────────────────────────────
    pub commit_message: String,
    pub commit_cursor: usize,
    pub amend_mode: bool,

    // ── UI modes ───────────────────────────────────────────────────────
    pub show_help: bool,
    pub show_command_palette: bool,
    pub command_palette_input: String,
    pub command_palette_selected: usize,

    // ── Command palette data ───────────────────────────────────────────
    pub palette_commands: Vec<PaletteCommand>,
    pub palette_input: String,
    pub palette_cursor: usize,
    pub palette_selected: usize,

    // ── Context menu ───────────────────────────────────────────────────
    pub context_menu: Option<ContextMenu>,

    // ── Dialog ─────────────────────────────────────────────────────────
    pub dialog: Option<Dialog>,

    // ── Input mode ─────────────────────────────────────────────────────
    pub input_mode: InputMode,

    // ── Notifications ──────────────────────────────────────────────────
    pub notification: Option<Notification>,

    // ── Lifecycle ──────────────────────────────────────────────────────
    pub should_quit: bool,

    // ── Async operation ────────────────────────────────────────────────
    pub async_status: Option<String>,
    pub async_op: Option<AsyncOp>,
}

impl App {
    // ── Constructor ────────────────────────────────────────────────────

    /// Create a new `App` by discovering the repository at `path` and loading
    /// the initial git state.
    pub fn new(path: PathBuf) -> Result<Self> {
        // Validate the path points at (or is inside) a git repository.
        let repo =
            git::repo::open_repo(&path).context("Failed to open git repository")?;
        let repo_path = git::repo::repo_workdir(&repo);

        let mut app = Self {
            repo_path,

            head: HeadInfo::default(),
            branches: Vec::new(),
            staged: Vec::new(),
            unstaged: Vec::new(),
            untracked: Vec::new(),
            conflicts: Vec::new(),
            stashes: Vec::new(),
            remotes: Vec::new(),
            log: Vec::new(),
            ahead: 0,
            behind: 0,

            focus: PanelFocus::Changes,
            branch_list_state: ListState::default(),
            staged_list_state: ListState::default(),
            changes_list_state: ListState::default(),
            stash_list_state: ListState::default(),
            log_list_state: ListState::default(),
            diff_scroll: 0,
            log_scroll: 0,

            current_diff: None,
            diff_mode: DiffMode::Unified,

            commit_message: String::new(),
            commit_cursor: 0,
            amend_mode: false,

            show_help: false,
            show_command_palette: false,
            command_palette_input: String::new(),
            command_palette_selected: 0,

            palette_commands: default_palette_commands(),
            palette_input: String::new(),
            palette_cursor: 0,
            palette_selected: 0,

            context_menu: None,

            dialog: None,

            input_mode: InputMode::Normal,

            notification: None,

            should_quit: false,

            async_status: None,
            async_op: None,
        };

        app.refresh()?;

        Ok(app)
    }

    // ── Helpers to open repo ───────────────────────────────────────────

    /// Open the repository (non-mutable borrow).
    fn open_repo(&self) -> Result<git2::Repository> {
        git::repo::open_repo(&self.repo_path)
    }

    /// Open the repository (returns owned, for stash ops that need `&mut`).
    fn open_repo_mut(&self) -> Result<git2::Repository> {
        git::repo::open_repo(&self.repo_path)
    }

    // ── Computed properties expected by UI ──────────────────────────────

    /// Combined list of changed files (unstaged + untracked) for the UI.
    pub fn changed_files(&self) -> Vec<&FileStatus> {
        self.unstaged.iter().chain(self.untracked.iter()).collect()
    }

    /// Current selection index in the changes panel.
    pub fn changes_index(&self) -> usize {
        self.changes_list_state.selected().unwrap_or(0)
    }

    /// Current selection index in the stash panel.
    pub fn stash_index(&self) -> usize {
        self.stash_list_state.selected().unwrap_or(0)
    }

    /// Current selection index in the log panel.
    pub fn log_index(&self) -> usize {
        self.log_list_state.selected().unwrap_or(0)
    }

    /// Whether the working tree is clean (nothing staged, unstaged, or untracked).
    pub fn is_clean(&self) -> bool {
        self.staged.is_empty()
            && self.unstaged.is_empty()
            && self.untracked.is_empty()
            && self.conflicts.is_empty()
    }

    /// Alias: the commit log as `commits` (UI uses this name).
    pub fn commits(&self) -> &[CommitInfo] {
        &self.log
    }

    // ── Refresh ────────────────────────────────────────────────────────

    /// Reload *all* git state from the repository on disk.
    pub fn refresh(&mut self) -> Result<()> {
        self.refresh_head()?;
        self.refresh_status()?;
        self.refresh_branches()?;
        self.refresh_log()?;
        self.refresh_stashes()?;
        self.refresh_remotes()?;
        self.refresh_ahead_behind()?;
        Ok(())
    }

    /// Reload HEAD information.
    fn refresh_head(&mut self) -> Result<()> {
        let repo = self.open_repo()?;
        self.head = git::repo::get_head_info(&repo)?;
        Ok(())
    }

    /// Reload the file status (staged, unstaged, untracked, conflicts).
    pub fn refresh_status(&mut self) -> Result<()> {
        let repo = self.open_repo()?;
        let (staged, unstaged, untracked, conflicts) = git::status::get_status(&repo)?;

        self.staged = staged;
        self.unstaged = unstaged;
        self.untracked = untracked;
        self.conflicts = conflicts;

        // Clamp list selections so they don't exceed new lengths.
        clamp_list_state(&mut self.staged_list_state, self.staged.len());
        let changes_len = self.unstaged.len() + self.untracked.len();
        clamp_list_state(&mut self.changes_list_state, changes_len);

        Ok(())
    }

    /// Reload branches.
    pub fn refresh_branches(&mut self) -> Result<()> {
        let repo = self.open_repo()?;
        self.branches = git::branch::list_branches(&repo)?;
        clamp_list_state(&mut self.branch_list_state, self.branches.len());
        Ok(())
    }

    /// Reload the commit log (last 500 commits).
    pub fn refresh_log(&mut self) -> Result<()> {
        let repo = self.open_repo()?;
        self.log = git::log::get_log(&repo, 500)?;
        clamp_list_state(&mut self.log_list_state, self.log.len());
        Ok(())
    }

    /// Reload stash entries.
    pub fn refresh_stashes(&mut self) -> Result<()> {
        let mut repo = self.open_repo_mut()?;
        self.stashes = git::stash::list_stashes(&mut repo)?;
        clamp_list_state(&mut self.stash_list_state, self.stashes.len());
        Ok(())
    }

    /// Reload remotes.
    fn refresh_remotes(&mut self) -> Result<()> {
        let repo = self.open_repo()?;
        self.remotes = git::remote::list_remotes(&repo)?;
        Ok(())
    }

    /// Reload ahead/behind counts.
    fn refresh_ahead_behind(&mut self) -> Result<()> {
        let repo = self.open_repo()?;
        let (ahead, behind) = git::remote::get_ahead_behind(&repo)?;
        self.ahead = ahead;
        self.behind = behind;
        Ok(())
    }

    // ── Changes helper ─────────────────────────────────────────────────

    /// Combined unstaged + untracked list length (the "Changes" panel).
    pub fn changes_len(&self) -> usize {
        self.unstaged.len() + self.untracked.len()
    }

    /// Get a file from the combined changes list by index.
    pub fn change_at(&self, idx: usize) -> Option<&FileStatus> {
        if idx < self.unstaged.len() {
            self.unstaged.get(idx)
        } else {
            self.untracked.get(idx - self.unstaged.len())
        }
    }

    // ── Diff ───────────────────────────────────────────────────────────

    /// Recompute the diff for the currently selected file in the focused panel.
    pub fn update_diff(&mut self) -> Result<()> {
        let repo = self.open_repo()?;

        match self.focus {
            PanelFocus::Staged => {
                if let Some(idx) = self.staged_list_state.selected() {
                    if let Some(file) = self.staged.get(idx) {
                        let diff = git::diff::get_staged_diff(&repo, &file.path)?;
                        self.current_diff = Some(diff);
                        self.diff_scroll = 0;
                    }
                } else {
                    self.current_diff = None;
                }
            }
            PanelFocus::Changes => {
                if let Some(idx) = self.changes_list_state.selected() {
                    if let Some(file) = self.change_at(idx) {
                        let diff = git::diff::get_unstaged_diff(&repo, &file.path)?;
                        self.current_diff = Some(diff);
                        self.diff_scroll = 0;
                    }
                } else {
                    self.current_diff = None;
                }
            }
            _ => {
                // No diff for other panels.
            }
        }

        Ok(())
    }

    /// Toggle between unified and side-by-side diff.
    pub fn toggle_diff_mode(&mut self) {
        self.diff_mode = match self.diff_mode {
            DiffMode::Unified => DiffMode::SideBySide,
            DiffMode::SideBySide => DiffMode::Unified,
        };
    }

    // ── Panel navigation ───────────────────────────────────────────────

    /// Move focus to the next panel (Tab).
    pub fn next_panel(&mut self) {
        self.focus = self.focus.next();
        if matches!(
            self.focus,
            PanelFocus::CommitMessage | PanelFocus::Commit
        ) {
            self.input_mode = InputMode::EditingCommitMessage;
        } else if matches!(
            self.input_mode,
            InputMode::EditingCommitMessage | InputMode::Editing
        ) {
            self.input_mode = InputMode::Normal;
        }
    }

    /// Move focus to the previous panel (Shift+Tab).
    pub fn prev_panel(&mut self) {
        self.focus = self.focus.prev();
        if matches!(
            self.focus,
            PanelFocus::CommitMessage | PanelFocus::Commit
        ) {
            self.input_mode = InputMode::EditingCommitMessage;
        } else if matches!(
            self.input_mode,
            InputMode::EditingCommitMessage | InputMode::Editing
        ) {
            self.input_mode = InputMode::Normal;
        }
    }

    /// Jump to a specific panel by number key (1-6).
    pub fn jump_to_panel(&mut self, n: u8) {
        if let Some(panel) = PanelFocus::from_number(n) {
            self.focus = panel;
            if matches!(
                self.focus,
                PanelFocus::CommitMessage | PanelFocus::Commit
            ) {
                self.input_mode = InputMode::EditingCommitMessage;
            } else if matches!(
                self.input_mode,
                InputMode::EditingCommitMessage | InputMode::Editing
            ) {
                self.input_mode = InputMode::Normal;
            }
        }
    }

    // ── List navigation ────────────────────────────────────────────────

    /// Move selection down in the currently focused list panel.
    pub fn select_next(&mut self) {
        match self.focus {
            PanelFocus::Branches => {
                let len = self.branches.len();
                advance_list_state(&mut self.branch_list_state, len);
            }
            PanelFocus::Staged => {
                let len = self.staged.len();
                advance_list_state(&mut self.staged_list_state, len);
            }
            PanelFocus::Changes => {
                let len = self.changes_len();
                advance_list_state(&mut self.changes_list_state, len);
            }
            PanelFocus::Stash => {
                let len = self.stashes.len();
                advance_list_state(&mut self.stash_list_state, len);
            }
            PanelFocus::LogGraph => {
                let len = self.log.len();
                advance_list_state(&mut self.log_list_state, len);
            }
            PanelFocus::DiffViewer => {
                self.diff_scroll = self.diff_scroll.saturating_add(1);
            }
            _ => {}
        }
    }

    /// Move selection up in the currently focused list panel.
    pub fn select_prev(&mut self) {
        match self.focus {
            PanelFocus::Branches => {
                let len = self.branches.len();
                retreat_list_state(&mut self.branch_list_state, len);
            }
            PanelFocus::Staged => {
                let len = self.staged.len();
                retreat_list_state(&mut self.staged_list_state, len);
            }
            PanelFocus::Changes => {
                let len = self.changes_len();
                retreat_list_state(&mut self.changes_list_state, len);
            }
            PanelFocus::Stash => {
                let len = self.stashes.len();
                retreat_list_state(&mut self.stash_list_state, len);
            }
            PanelFocus::LogGraph => {
                let len = self.log.len();
                retreat_list_state(&mut self.log_list_state, len);
            }
            PanelFocus::DiffViewer => {
                self.diff_scroll = self.diff_scroll.saturating_sub(1);
            }
            _ => {}
        }
    }

    /// Select the first item in the currently focused list.
    pub fn select_first(&mut self) {
        match self.focus {
            PanelFocus::Branches => {
                let len = self.branches.len();
                select_first(&mut self.branch_list_state, len);
            }
            PanelFocus::Staged => {
                let len = self.staged.len();
                select_first(&mut self.staged_list_state, len);
            }
            PanelFocus::Changes => {
                let len = self.changes_len();
                select_first(&mut self.changes_list_state, len);
            }
            PanelFocus::Stash => {
                let len = self.stashes.len();
                select_first(&mut self.stash_list_state, len);
            }
            PanelFocus::LogGraph => {
                let len = self.log.len();
                select_first(&mut self.log_list_state, len);
            }
            PanelFocus::DiffViewer => self.diff_scroll = 0,
            _ => {}
        }
    }

    /// Select the last item in the currently focused list.
    pub fn select_last(&mut self) {
        match self.focus {
            PanelFocus::Branches => {
                let len = self.branches.len();
                select_last(&mut self.branch_list_state, len);
            }
            PanelFocus::Staged => {
                let len = self.staged.len();
                select_last(&mut self.staged_list_state, len);
            }
            PanelFocus::Changes => {
                let len = self.changes_len();
                select_last(&mut self.changes_list_state, len);
            }
            PanelFocus::Stash => {
                let len = self.stashes.len();
                select_last(&mut self.stash_list_state, len);
            }
            PanelFocus::LogGraph => {
                let len = self.log.len();
                select_last(&mut self.log_list_state, len);
            }
            _ => {}
        }
    }

    // ── Git operations ─────────────────────────────────────────────────

    /// Stage the currently selected file in the Changes panel.
    pub fn stage_selected(&mut self) -> Result<()> {
        if self.focus != PanelFocus::Changes {
            return Ok(());
        }
        let idx = match self.changes_list_state.selected() {
            Some(i) => i,
            None => return Ok(()),
        };
        let path = match self.change_at(idx) {
            Some(f) => f.path.clone(),
            None => return Ok(()),
        };

        let repo = self.open_repo()?;
        git::status::stage_file(&repo, &path)?;
        self.refresh_status()?;
        self.notify("Staged".to_string(), NotificationType::Success);
        Ok(())
    }

    /// Unstage the currently selected file in the Staged panel.
    pub fn unstage_selected(&mut self) -> Result<()> {
        if self.focus != PanelFocus::Staged {
            return Ok(());
        }
        let idx = match self.staged_list_state.selected() {
            Some(i) => i,
            None => return Ok(()),
        };
        let path = match self.staged.get(idx) {
            Some(f) => f.path.clone(),
            None => return Ok(()),
        };

        let repo = self.open_repo()?;
        git::status::unstage_file(&repo, &path)?;
        self.refresh_status()?;
        self.notify("Unstaged".to_string(), NotificationType::Success);
        Ok(())
    }

    /// Discard changes for the currently selected file (with confirmation dialog).
    pub fn discard_selected(&mut self) -> Result<()> {
        if self.focus != PanelFocus::Changes {
            return Ok(());
        }
        let idx = match self.changes_list_state.selected() {
            Some(i) => i,
            None => return Ok(()),
        };
        let path = match self.change_at(idx) {
            Some(f) => f.path.clone(),
            None => return Ok(()),
        };

        self.dialog = Some(Dialog {
            title: "Discard Changes".to_string(),
            message: format!("Discard all changes to '{}'?", path),
            dialog_type: DialogType::Confirm {
                on_confirm: DialogAction::DiscardChanges(path),
            },
            input: String::new(),
            selected: 0,
        });
        self.input_mode = InputMode::DialogInput;
        Ok(())
    }

    /// Actually perform the discard (called after dialog confirmation).
    pub fn do_discard_changes(&mut self, path: &str) -> Result<()> {
        let repo = self.open_repo()?;
        git::status::discard_changes(&repo, path)?;
        self.refresh_status()?;
        self.notify("Changes discarded".to_string(), NotificationType::Success);
        Ok(())
    }

    /// Perform a commit with the current message.
    pub fn do_commit(&mut self) -> Result<()> {
        let msg = self.commit_message.trim().to_string();
        if msg.is_empty() {
            self.notify(
                "Commit message is empty".to_string(),
                NotificationType::Warning,
            );
            return Ok(());
        }
        if self.staged.is_empty() {
            self.notify(
                "Nothing staged to commit".to_string(),
                NotificationType::Warning,
            );
            return Ok(());
        }

        let repo = self.open_repo()?;
        if self.amend_mode {
            git::commit::amend(&repo, &msg)?;
            self.notify("Amended commit".to_string(), NotificationType::Success);
        } else {
            git::commit::commit(&repo, &msg)?;
            self.notify("Created commit".to_string(), NotificationType::Success);
        }

        self.commit_message.clear();
        self.commit_cursor = 0;
        self.amend_mode = false;
        self.refresh()?;
        Ok(())
    }

    /// Toggle amend mode. When entering amend mode, pre-fill the commit
    /// message with the HEAD commit's summary.
    pub fn toggle_amend(&mut self) {
        self.amend_mode = !self.amend_mode;
        if self.amend_mode {
            self.commit_message = self.head.summary.clone();
            self.commit_cursor = self.commit_message.len();
            self.notify("Amend mode ON".to_string(), NotificationType::Info);
        } else {
            self.notify("Amend mode OFF".to_string(), NotificationType::Info);
        }
    }

    /// Stash working changes.
    pub fn stash_save(&mut self) -> Result<()> {
        self.dialog = Some(Dialog {
            title: "Stash Save".to_string(),
            message: "Enter stash message (optional):".to_string(),
            dialog_type: DialogType::Input {
                on_submit: DialogAction::StashSave,
            },
            input: String::new(),
            selected: 0,
        });
        self.input_mode = InputMode::DialogInput;
        Ok(())
    }

    /// Actually save a stash with the given message.
    pub fn do_stash_save(&mut self, message: &str) -> Result<()> {
        let mut repo = self.open_repo_mut()?;
        let msg = if message.is_empty() {
            None
        } else {
            Some(message)
        };
        git::stash::stash_save(&mut repo, msg)?;
        self.refresh()?;
        self.notify("Stash saved".to_string(), NotificationType::Success);
        Ok(())
    }

    /// Pop the most recent stash.
    pub fn stash_pop(&mut self) -> Result<()> {
        let mut repo = self.open_repo_mut()?;
        git::stash::stash_pop(&mut repo)?;
        self.refresh()?;
        self.notify("Stash popped".to_string(), NotificationType::Success);
        Ok(())
    }

    /// Drop a stash by index (with confirmation dialog).
    pub fn stash_drop_selected(&mut self) -> Result<()> {
        if let Some(idx) = self.stash_list_state.selected() {
            if idx < self.stashes.len() {
                self.dialog = Some(Dialog {
                    title: "Drop Stash".to_string(),
                    message: format!("Drop stash@{{{}}}?", idx),
                    dialog_type: DialogType::Confirm {
                        on_confirm: DialogAction::StashDrop(idx),
                    },
                    input: String::new(),
                    selected: 0,
                });
                self.input_mode = InputMode::DialogInput;
            }
        }
        Ok(())
    }

    /// Actually drop a stash.
    pub fn do_stash_drop(&mut self, index: usize) -> Result<()> {
        let mut repo = self.open_repo_mut()?;
        git::stash::stash_drop(&mut repo, index)?;
        self.refresh_stashes()?;
        self.notify("Stash dropped".to_string(), NotificationType::Success);
        Ok(())
    }

    /// Create a new branch (opens dialog).
    pub fn create_branch(&mut self) -> Result<()> {
        self.dialog = Some(Dialog {
            title: "Create Branch".to_string(),
            message: "Enter new branch name:".to_string(),
            dialog_type: DialogType::Input {
                on_submit: DialogAction::CreateBranch,
            },
            input: String::new(),
            selected: 0,
        });
        self.input_mode = InputMode::DialogInput;
        Ok(())
    }

    /// Actually create a branch with the given name.
    pub fn do_create_branch(&mut self, name: &str) -> Result<()> {
        let repo = self.open_repo()?;
        git::branch::create_branch(&repo, name)?;
        self.refresh_branches()?;
        self.notify(
            format!("Branch '{}' created", name),
            NotificationType::Success,
        );
        Ok(())
    }

    /// Delete the currently selected branch (with confirmation).
    pub fn delete_selected_branch(&mut self) -> Result<()> {
        if let Some(idx) = self.branch_list_state.selected() {
            if let Some(branch) = self.branches.get(idx) {
                if branch.is_current {
                    self.notify(
                        "Cannot delete the current branch".to_string(),
                        NotificationType::Error,
                    );
                    return Ok(());
                }
                let name = branch.name.clone();
                self.dialog = Some(Dialog {
                    title: "Delete Branch".to_string(),
                    message: format!("Delete branch '{}'?", name),
                    dialog_type: DialogType::Confirm {
                        on_confirm: DialogAction::DeleteBranch(name),
                    },
                    input: String::new(),
                    selected: 0,
                });
                self.input_mode = InputMode::DialogInput;
            }
        }
        Ok(())
    }

    /// Actually delete a branch.
    pub fn do_delete_branch(&mut self, name: &str) -> Result<()> {
        let repo = self.open_repo()?;
        git::branch::delete_branch(&repo, name)?;
        self.refresh_branches()?;
        self.notify(
            format!("Branch '{}' deleted", name),
            NotificationType::Success,
        );
        Ok(())
    }

    /// Checkout the currently selected branch.
    pub fn checkout_selected_branch(&mut self) -> Result<()> {
        if self.focus != PanelFocus::Branches {
            return Ok(());
        }
        if let Some(idx) = self.branch_list_state.selected() {
            if let Some(branch) = self.branches.get(idx) {
                if branch.is_current {
                    return Ok(());
                }
                let name = branch.name.clone();
                let repo = self.open_repo()?;
                git::branch::checkout_branch(&repo, &name)?;
                self.refresh()?;
                self.notify(
                    format!("Checked out '{}'", name),
                    NotificationType::Success,
                );
            }
        }
        Ok(())
    }

    // ── Dialog handling ────────────────────────────────────────────────

    /// Confirm the current dialog action.
    pub fn confirm_dialog(&mut self) -> Result<()> {
        if let Some(dialog) = self.dialog.take() {
            self.input_mode = InputMode::Normal;
            match dialog.dialog_type {
                DialogType::Confirm { on_confirm } => {
                    self.execute_dialog_action(on_confirm, &dialog.input)?;
                }
                DialogType::Input { on_submit } => {
                    self.execute_dialog_action(on_submit, &dialog.input)?;
                }
                DialogType::Select { on_select, .. } => {
                    self.execute_dialog_action(on_select, &dialog.input)?;
                }
            }
        }
        Ok(())
    }

    /// Cancel/dismiss the current dialog.
    pub fn cancel_dialog(&mut self) {
        self.dialog = None;
        self.input_mode = InputMode::Normal;
    }

    /// Execute a dialog action with the provided input.
    fn execute_dialog_action(&mut self, action: DialogAction, input: &str) -> Result<()> {
        match action {
            DialogAction::DeleteBranch(name) => self.do_delete_branch(&name)?,
            DialogAction::DiscardChanges(path) => self.do_discard_changes(&path)?,
            DialogAction::ForcePush => {
                self.notify(
                    "Force push initiated".to_string(),
                    NotificationType::Info,
                );
            }
            DialogAction::CreateBranch => self.do_create_branch(input)?,
            DialogAction::RenameBranch(old) => {
                let repo = self.open_repo()?;
                git::branch::rename_branch(&repo, &old, input)?;
                self.refresh_branches()?;
                self.notify(
                    format!("Branch '{}' renamed to '{}'", old, input),
                    NotificationType::Success,
                );
            }
            DialogAction::StashSave => self.do_stash_save(input)?,
            DialogAction::StashDrop(idx) => self.do_stash_drop(idx)?,
            DialogAction::CheckoutBranch(name) => {
                let repo = self.open_repo()?;
                git::branch::checkout_branch(&repo, &name)?;
                self.refresh()?;
                self.notify(
                    format!("Checked out '{}'", name),
                    NotificationType::Success,
                );
            }
        }
        Ok(())
    }

    // ── Command palette ────────────────────────────────────────────────

    /// Open the command palette.
    pub fn open_command_palette(&mut self) {
        self.show_command_palette = true;
        self.command_palette_input.clear();
        self.palette_input.clear();
        self.palette_cursor = 0;
        self.command_palette_selected = 0;
        self.palette_selected = 0;
        self.input_mode = InputMode::CommandPalette;
    }

    /// Close the command palette.
    pub fn close_command_palette(&mut self) {
        self.show_command_palette = false;
        self.command_palette_input.clear();
        self.palette_input.clear();
        self.input_mode = InputMode::Normal;
    }

    // ── Help ───────────────────────────────────────────────────────────

    /// Toggle the help overlay.
    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }

    // ── Notifications ──────────────────────────────────────────────────

    /// Display a notification that auto-dismisses after a few seconds.
    pub fn notify(&mut self, msg: String, ntype: NotificationType) {
        let is_error = matches!(ntype, NotificationType::Error);
        self.notification = Some(Notification {
            message: msg,
            is_error,
            when: Instant::now(),
        });
    }

    /// Clear the notification if it has expired (>3 seconds old).
    pub fn tick_notification(&mut self) {
        if let Some(ref notif) = self.notification {
            if notif.when.elapsed().as_secs() >= 3 {
                self.notification = None;
            }
        }
    }

    // ── Focus commit ───────────────────────────────────────────────────

    /// Focus the commit message editor.
    pub fn focus_commit_message(&mut self) {
        self.focus = PanelFocus::CommitMessage;
        self.input_mode = InputMode::EditingCommitMessage;
    }

    /// Leave the commit message editor and return to normal mode.
    pub fn leave_commit_message(&mut self) {
        if matches!(
            self.input_mode,
            InputMode::EditingCommitMessage | InputMode::Editing
        ) {
            self.input_mode = InputMode::Normal;
            self.focus = PanelFocus::Changes;
        }
    }
}

// ---------------------------------------------------------------------------
// List state helpers
// ---------------------------------------------------------------------------

/// Advance the selection in a `ListState` by one, wrapping around.
fn advance_list_state(state: &mut ListState, len: usize) {
    if len == 0 {
        state.select(None);
        return;
    }
    let next = match state.selected() {
        Some(i) => {
            if i >= len - 1 {
                0
            } else {
                i + 1
            }
        }
        None => 0,
    };
    state.select(Some(next));
}

/// Retreat the selection in a `ListState` by one, wrapping around.
fn retreat_list_state(state: &mut ListState, len: usize) {
    if len == 0 {
        state.select(None);
        return;
    }
    let prev = match state.selected() {
        Some(i) => {
            if i == 0 {
                len - 1
            } else {
                i - 1
            }
        }
        None => len.saturating_sub(1),
    };
    state.select(Some(prev));
}

/// Clamp a `ListState` selection to be within `[0, len)`.
fn clamp_list_state(state: &mut ListState, len: usize) {
    if len == 0 {
        state.select(None);
    } else if let Some(i) = state.selected() {
        if i >= len {
            state.select(Some(len - 1));
        }
    }
}

/// Select the first item.
fn select_first(state: &mut ListState, len: usize) {
    if len > 0 {
        state.select(Some(0));
    }
}

/// Select the last item.
fn select_last(state: &mut ListState, len: usize) {
    if len > 0 {
        state.select(Some(len - 1));
    }
}
