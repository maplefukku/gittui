pub mod branch;
pub mod commit;
pub mod conflict;
pub mod diff;
pub mod log;
pub mod remote;
pub mod repo;
pub mod stash;
pub mod status;

// Re-export primary types for convenient access.

pub use branch::BranchInfo;
pub use commit::{amend, commit};
pub use conflict::{ConflictEntry, ConflictSide};
pub use diff::{DiffLine, DiffLineType, FileDiff, HunkInfo};
pub use log::{CommitInfo, GraphSymbol};
pub use remote::{RemoteInfo, SyncStatus};
pub use repo::HeadInfo;
pub use stash::StashInfo;
pub use status::{FileStatus, StatusType};
