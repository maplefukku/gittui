use anyhow::{bail, Context, Result};
use git2::{BranchType, Repository};

/// Information about a single branch.
#[derive(Debug, Clone)]
pub struct BranchInfo {
    /// Branch name.
    pub name: String,
    /// Whether this is the currently checked-out branch.
    pub is_current: bool,
    /// Whether this is a remote-tracking branch.
    pub is_remote: bool,
    /// The upstream branch name, if any.
    pub upstream: Option<String>,
    /// The OID of the branch tip commit.
    pub oid: String,
    /// Short commit summary at the tip.
    pub summary: String,
    /// Number of commits ahead of upstream.
    pub ahead: usize,
    /// Number of commits behind upstream.
    pub behind: usize,
    /// Last commit message (first line) - alias kept for compatibility.
    pub last_commit_msg: Option<String>,
}

/// List all local branches, sorted alphabetically with the current branch
/// first.
pub fn list_branches(repo: &Repository) -> Result<Vec<BranchInfo>> {
    let branches = repo
        .branches(Some(BranchType::Local))
        .context("failed to list branches")?;

    let head = repo.head().ok();
    let head_oid = head.as_ref().and_then(|h| h.target());

    let mut infos: Vec<BranchInfo> = Vec::new();

    for branch_result in branches {
        let (branch, _btype) = branch_result.context("failed to read branch entry")?;
        let name = branch
            .name()
            .context("failed to read branch name")?
            .unwrap_or("<invalid utf-8>")
            .to_string();

        let reference = branch.get();
        let oid_val = reference.target();
        let oid = oid_val.map(|o| o.to_string()).unwrap_or_default();

        let is_current = head_oid
            .map(|h| Some(h) == oid_val)
            .unwrap_or(false);

        let summary = if let Ok(commit) = reference.peel_to_commit() {
            commit.summary().unwrap_or("").to_string()
        } else {
            String::new()
        };

        let upstream = branch
            .upstream()
            .ok()
            .and_then(|u| u.name().ok().flatten().map(String::from));

        let (ahead, behind) = if let (Some(local_oid), Ok(upstream_branch)) =
            (oid_val, branch.upstream())
        {
            if let Some(remote_oid) = upstream_branch.get().target() {
                repo.graph_ahead_behind(local_oid, remote_oid)
                    .unwrap_or((0, 0))
            } else {
                (0, 0)
            }
        } else {
            (0, 0)
        };

        infos.push(BranchInfo {
            name,
            is_current,
            is_remote: false,
            upstream,
            oid,
            summary: summary.clone(),
            ahead,
            behind,
            last_commit_msg: if summary.is_empty() { None } else { Some(summary) },
        });
    }

    // Sort: current branch first, then alphabetical.
    infos.sort_by(|a, b| {
        b.is_current
            .cmp(&a.is_current)
            .then_with(|| a.name.cmp(&b.name))
    });

    Ok(infos)
}

/// List remote-tracking branches.
pub fn list_remote_branches(repo: &Repository) -> Result<Vec<BranchInfo>> {
    let branches = repo
        .branches(Some(BranchType::Remote))
        .context("failed to list remote branches")?;

    let mut infos: Vec<BranchInfo> = Vec::new();

    for branch_result in branches {
        let (branch, _) = branch_result.context("failed to read remote branch entry")?;
        let name = branch
            .name()
            .context("failed to read remote branch name")?
            .unwrap_or("")
            .to_string();

        let reference = branch.get();
        let oid = reference.target().map(|o| o.to_string()).unwrap_or_default();

        let summary = if let Ok(commit) = reference.peel_to_commit() {
            commit.summary().unwrap_or("").to_string()
        } else {
            String::new()
        };

        infos.push(BranchInfo {
            name,
            is_current: false,
            is_remote: true,
            upstream: None,
            oid,
            summary: summary.clone(),
            ahead: 0,
            behind: 0,
            last_commit_msg: if summary.is_empty() { None } else { Some(summary) },
        });
    }

    infos.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(infos)
}

/// Create a new local branch pointing at the current HEAD.
pub fn create_branch(repo: &Repository, name: &str) -> Result<()> {
    let head = repo
        .head()
        .context("HEAD not found")?;
    let commit = head
        .peel_to_commit()
        .context("HEAD does not point to a commit")?;
    repo.branch(name, &commit, false)
        .with_context(|| format!("failed to create branch '{name}'"))?;
    Ok(())
}

/// Create a branch at a specific commit with an optional force flag.
pub fn create_branch_at<'repo>(
    repo: &'repo Repository,
    name: &str,
    oid: git2::Oid,
    force: bool,
) -> Result<git2::Branch<'repo>> {
    let commit = repo
        .find_commit(oid)
        .with_context(|| format!("commit {oid} not found"))?;
    repo.branch(name, &commit, force)
        .with_context(|| format!("failed to create branch '{name}' at {oid}"))
}

/// Delete a local branch by name.
///
/// Refuses to delete the currently checked-out branch.
pub fn delete_branch(repo: &Repository, name: &str) -> Result<()> {
    let head_name = repo
        .head()
        .ok()
        .and_then(|h| h.shorthand().map(String::from));

    if head_name.as_deref() == Some(name) {
        bail!("cannot delete the currently checked-out branch '{name}'");
    }

    let mut branch = repo
        .find_branch(name, BranchType::Local)
        .with_context(|| format!("branch '{name}' not found"))?;
    branch
        .delete()
        .with_context(|| format!("failed to delete branch '{name}'"))?;
    Ok(())
}

/// Rename a local branch.
pub fn rename_branch(repo: &Repository, old_name: &str, new_name: &str) -> Result<()> {
    let mut branch = repo
        .find_branch(old_name, BranchType::Local)
        .with_context(|| format!("branch '{old_name}' not found"))?;
    branch
        .rename(new_name, false)
        .with_context(|| format!("failed to rename branch '{old_name}' to '{new_name}'"))?;
    Ok(())
}

/// Rename a local branch with a force option.
pub fn rename_branch_force(
    repo: &Repository,
    old_name: &str,
    new_name: &str,
    force: bool,
) -> Result<()> {
    let mut branch = repo
        .find_branch(old_name, BranchType::Local)
        .with_context(|| format!("branch '{old_name}' not found"))?;
    branch
        .rename(new_name, force)
        .with_context(|| format!("failed to rename branch '{old_name}' to '{new_name}'"))?;
    Ok(())
}

/// Checkout a branch by name.
///
/// Performs a safe checkout by default.
pub fn checkout_branch(repo: &Repository, name: &str) -> Result<()> {
    let refname = format!("refs/heads/{name}");
    let obj = repo
        .revparse_single(&refname)
        .with_context(|| format!("branch '{name}' not found"))?;
    repo.checkout_tree(&obj, None)
        .with_context(|| format!("failed to checkout tree for branch '{name}'"))?;
    repo.set_head(&refname)
        .with_context(|| format!("failed to set HEAD to branch '{name}'"))?;
    Ok(())
}

/// Checkout a specific commit (detached HEAD).
pub fn checkout_commit(repo: &Repository, oid: git2::Oid) -> Result<()> {
    let commit = repo
        .find_commit(oid)
        .with_context(|| format!("commit {oid} not found"))?;
    repo.checkout_tree(commit.as_object(), None)
        .context("failed to checkout commit tree")?;
    repo.set_head_detached(oid)
        .context("failed to detach HEAD")?;
    Ok(())
}

/// Set the upstream tracking branch for a local branch.
pub fn set_upstream(repo: &Repository, branch_name: &str, upstream: &str) -> Result<()> {
    let mut branch = repo
        .find_branch(branch_name, BranchType::Local)
        .with_context(|| format!("branch '{branch_name}' not found"))?;
    branch
        .set_upstream(Some(upstream))
        .with_context(|| format!("failed to set upstream of '{branch_name}' to '{upstream}'"))
}

/// Remove the upstream tracking configuration for a local branch.
pub fn unset_upstream(repo: &Repository, branch_name: &str) -> Result<()> {
    let mut branch = repo
        .find_branch(branch_name, BranchType::Local)
        .with_context(|| format!("branch '{branch_name}' not found"))?;
    branch
        .set_upstream(None)
        .with_context(|| format!("failed to unset upstream of '{branch_name}'"))
}
