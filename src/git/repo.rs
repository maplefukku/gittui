use std::path::PathBuf;

use anyhow::{Context, Result};
use git2::Repository;

/// Information about the current HEAD reference.
#[derive(Debug, Clone)]
#[derive(Default)]
pub struct HeadInfo {
    /// Name of the current branch, or `None` if HEAD is detached.
    pub branch: Option<String>,
    /// The full OID hex string of the HEAD commit (empty for unborn branches).
    pub oid: String,
    /// Short summary (first line) of the HEAD commit.
    pub summary: String,
    /// Whether HEAD is in a detached state.
    pub detached: bool,
}


/// Open the git repository at (or containing) the given path.
///
/// Walks parent directories until it finds a `.git` folder, matching the
/// behaviour of the `git` CLI (`git2::Repository::discover`).
pub fn open_repo(path: &std::path::Path) -> Result<Repository> {
    Repository::discover(path)
        .with_context(|| format!("failed to open git repository at {}", path.display()))
}

/// Discover and open the repository starting from the current working
/// directory.
pub fn open_cwd_repo() -> Result<Repository> {
    let cwd = std::env::current_dir().context("failed to determine current working directory")?;
    open_repo(&cwd)
}

/// Get HEAD information from the repository.
pub fn get_head_info(repo: &Repository) -> Result<HeadInfo> {
    let head = match repo.head() {
        Ok(h) => h,
        Err(e) if e.code() == git2::ErrorCode::UnbornBranch => {
            // Empty repo with no commits — try to read the unborn branch name.
            let branch_name = repo
                .find_reference("HEAD")
                .ok()
                .and_then(|r| r.symbolic_target().map(String::from))
                .and_then(|s| s.strip_prefix("refs/heads/").map(String::from));
            return Ok(HeadInfo {
                branch: branch_name,
                oid: String::new(),
                summary: String::new(),
                detached: false,
            });
        }
        Err(_) => {
            return Ok(HeadInfo::default());
        }
    };

    let detached = repo.head_detached().unwrap_or(false);
    let branch = if detached {
        None
    } else {
        head.shorthand().map(|s| s.to_string())
    };

    let oid = head
        .target()
        .map(|o| o.to_string())
        .unwrap_or_default();

    let summary = if let Ok(commit) = head.peel_to_commit() {
        commit.summary().unwrap_or("").to_string()
    } else {
        String::new()
    };

    Ok(HeadInfo {
        branch,
        oid,
        summary,
        detached,
    })
}

/// Get the workdir path for a repository.
///
/// For bare repositories, falls back to the `.git` directory path.
pub fn repo_workdir(repo: &Repository) -> PathBuf {
    repo.workdir()
        .unwrap_or_else(|| repo.path())
        .to_path_buf()
}

/// Return the short (abbreviated) hex string for an OID.
pub fn short_oid(repo: &Repository, oid: git2::Oid) -> String {
    repo.find_object(oid, None)
        .ok()
        .and_then(|obj| obj.short_id().ok())
        .and_then(|buf| buf.as_str().map(String::from))
        .unwrap_or_else(|| {
            let s = oid.to_string();
            s[..7.min(s.len())].to_string()
        })
}

/// Return `true` if the repository has any commits at all.
pub fn has_commits(repo: &Repository) -> bool {
    repo.head().is_ok()
}

/// Return the repository state (normal, merge, rebase, etc.).
pub fn repo_state(repo: &Repository) -> git2::RepositoryState {
    repo.state()
}
