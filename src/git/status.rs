use anyhow::{Context, Result};
use git2::{Repository, StatusOptions, StatusShow};

/// The kind of change for a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatusType {
    New,
    /// Alias for `New` used by some UI code.
    Added,
    Modified,
    Deleted,
    Renamed,
    Copied,
    TypeChange,
    Untracked,
    Conflicted,
}

impl std::fmt::Display for StatusType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StatusType::New => write!(f, "new"),
            StatusType::Added => write!(f, "added"),
            StatusType::Modified => write!(f, "modified"),
            StatusType::Deleted => write!(f, "deleted"),
            StatusType::Renamed => write!(f, "renamed"),
            StatusType::Copied => write!(f, "copied"),
            StatusType::TypeChange => write!(f, "typechange"),
            StatusType::Untracked => write!(f, "untracked"),
            StatusType::Conflicted => write!(f, "conflicted"),
        }
    }
}

/// Status of a single file in the working tree or index.
#[derive(Debug, Clone)]
pub struct FileStatus {
    /// Relative path from workdir root.
    pub path: String,
    /// Old path (for renames).
    pub old_path: Option<String>,
    /// The kind of change.
    pub status: StatusType,
}

/// The four-way grouping returned by [`get_status`]:
/// `(staged, unstaged, untracked, conflicted)`.
pub type StatusGroups = (Vec<FileStatus>, Vec<FileStatus>, Vec<FileStatus>, Vec<FileStatus>);

/// Retrieve file statuses grouped into staged, unstaged, untracked, and
/// conflicted.
///
/// Returns `(staged, unstaged, untracked, conflicted)`.
pub fn get_status(repo: &Repository) -> Result<StatusGroups> {
    let mut opts = StatusOptions::new();
    opts.include_untracked(true)
        .recurse_untracked_dirs(true)
        .include_ignored(false)
        .renames_head_to_index(true)
        .renames_index_to_workdir(true)
        .show(StatusShow::IndexAndWorkdir);

    let statuses = repo
        .statuses(Some(&mut opts))
        .context("failed to read repository status")?;

    let mut staged = Vec::new();
    let mut unstaged = Vec::new();
    let mut untracked = Vec::new();
    let mut conflicts = Vec::new();

    for entry in statuses.iter() {
        let path = entry
            .path()
            .unwrap_or("<invalid utf-8>")
            .to_string();
        let status = entry.status();

        // --- Conflicted -------------------------------------------------
        if status.is_conflicted() {
            conflicts.push(FileStatus {
                path: path.clone(),
                old_path: None,
                status: StatusType::Conflicted,
            });
            // A conflicted file may also have index/workdir bits set;
            // report it only once in the conflicted bucket.
            continue;
        }

        // --- Index (staged) changes -------------------------------------
        if status.is_index_new() {
            staged.push(FileStatus {
                path: path.clone(),
                old_path: None,
                status: StatusType::New,
            });
        } else if status.is_index_modified() {
            staged.push(FileStatus {
                path: path.clone(),
                old_path: None,
                status: StatusType::Modified,
            });
        } else if status.is_index_deleted() {
            staged.push(FileStatus {
                path: path.clone(),
                old_path: None,
                status: StatusType::Deleted,
            });
        } else if status.is_index_renamed() {
            let old = entry
                .head_to_index()
                .and_then(|d| d.old_file().path().map(|p| p.to_string_lossy().to_string()));
            staged.push(FileStatus {
                path: path.clone(),
                old_path: old,
                status: StatusType::Renamed,
            });
        } else if status.is_index_typechange() {
            staged.push(FileStatus {
                path: path.clone(),
                old_path: None,
                status: StatusType::TypeChange,
            });
        }

        // --- Workdir (unstaged) changes ---------------------------------
        if status.is_wt_modified() {
            unstaged.push(FileStatus {
                path: path.clone(),
                old_path: None,
                status: StatusType::Modified,
            });
        } else if status.is_wt_deleted() {
            unstaged.push(FileStatus {
                path: path.clone(),
                old_path: None,
                status: StatusType::Deleted,
            });
        } else if status.is_wt_renamed() {
            let old = entry
                .index_to_workdir()
                .and_then(|d| d.old_file().path().map(|p| p.to_string_lossy().to_string()));
            unstaged.push(FileStatus {
                path: path.clone(),
                old_path: old,
                status: StatusType::Renamed,
            });
        } else if status.is_wt_typechange() {
            unstaged.push(FileStatus {
                path: path.clone(),
                old_path: None,
                status: StatusType::TypeChange,
            });
        } else if status.is_wt_new() {
            untracked.push(FileStatus {
                path,
                old_path: None,
                status: StatusType::Untracked,
            });
        }
    }

    Ok((staged, unstaged, untracked, conflicts))
}

/// Stage a single file (add to index).
///
/// If the file has been deleted from the working tree, this removes it from
/// the index instead (matching `git add` behaviour).
pub fn stage_file(repo: &Repository, path: &str) -> Result<()> {
    let mut index = repo.index().context("failed to open index")?;
    let workdir = repo.workdir().context("repository is bare")?;
    let full_path = workdir.join(path);

    if full_path.exists() {
        index
            .add_path(std::path::Path::new(path))
            .with_context(|| format!("failed to stage {path}"))?;
    } else {
        index
            .remove_path(std::path::Path::new(path))
            .with_context(|| format!("failed to stage removal of {path}"))?;
    }
    index.write().context("failed to write index")?;
    Ok(())
}

/// Unstage a single file (remove from index, restore to HEAD state).
///
/// For files that exist in HEAD this resets the index entry to HEAD's version.
/// For files not in HEAD (newly added) this removes them from the index.
pub fn unstage_file(repo: &Repository, path: &str) -> Result<()> {
    let head_tree = repo
        .head()
        .ok()
        .and_then(|h| h.peel_to_tree().ok());

    match head_tree {
        Some(tree) => {
            repo.reset_default(Some(&tree.into_object()), [path])
                .with_context(|| format!("failed to unstage {path}"))?;
        }
        None => {
            // No HEAD yet (initial commit) — remove from index entirely.
            let mut index = repo.index().context("failed to open index")?;
            index
                .remove_path(std::path::Path::new(path))
                .with_context(|| format!("failed to unstage {path}"))?;
            index.write().context("failed to write index")?;
        }
    }

    Ok(())
}

/// Stage all files (equivalent to `git add -A`).
pub fn stage_all(repo: &Repository) -> Result<()> {
    let mut index = repo.index().context("failed to open index")?;
    index
        .add_all(["*"], git2::IndexAddOption::DEFAULT, None)
        .context("failed to stage all files")?;
    index.write().context("failed to write index")?;
    Ok(())
}

/// Discard working directory changes for a file (checkout from index).
pub fn discard_changes(repo: &Repository, path: &str) -> Result<()> {
    let mut opts = git2::build::CheckoutBuilder::new();
    opts.path(path).force();
    repo.checkout_index(None, Some(&mut opts))
        .with_context(|| format!("failed to discard changes in {path}"))?;
    Ok(())
}
