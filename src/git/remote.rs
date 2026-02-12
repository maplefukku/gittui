use anyhow::{Context, Result};
use git2::Repository;
use std::process::Command;

/// Summary of a configured remote.
#[derive(Debug, Clone)]
pub struct RemoteInfo {
    /// Remote name (e.g. "origin").
    pub name: String,
    /// Fetch URL.
    pub url: String,
    /// Push URL (may differ from fetch URL).
    pub push_url: Option<String>,
    /// Sync status relative to upstream (populated after computing ahead/behind).
    pub sync_status: SyncStatus,
}

/// Sync status relative to the upstream tracking branch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Default)]
pub enum SyncStatus {
    UpToDate,
    Ahead(usize),
    Behind(usize),
    Diverged { ahead: usize, behind: usize },
    #[default]
    Unknown,
}


impl SyncStatus {
    /// Create from ahead/behind counts.
    pub fn from_counts(ahead: usize, behind: usize) -> Self {
        match (ahead, behind) {
            (0, 0) => SyncStatus::UpToDate,
            (a, 0) => SyncStatus::Ahead(a),
            (0, b) => SyncStatus::Behind(b),
            (a, b) => SyncStatus::Diverged { ahead: a, behind: b },
        }
    }
}

/// List all remotes configured in the repository.
pub fn list_remotes(repo: &Repository) -> Result<Vec<RemoteInfo>> {
    let remote_names = repo
        .remotes()
        .context("failed to list remotes")?;

    let mut remotes = Vec::new();

    for name in remote_names.iter().flatten() {
        if let Ok(remote) = repo.find_remote(name) {
            remotes.push(RemoteInfo {
                name: name.to_string(),
                url: remote.url().unwrap_or("").to_string(),
                push_url: remote.pushurl().map(|s| s.to_string()),
                sync_status: SyncStatus::Unknown,
            });
        }
    }

    Ok(remotes)
}

/// Retrieve a single remote by name.
pub fn get_remote(repo: &Repository, name: &str) -> Result<RemoteInfo> {
    let remote = repo
        .find_remote(name)
        .with_context(|| format!("remote '{name}' not found"))?;

    Ok(RemoteInfo {
        name: name.to_string(),
        url: remote.url().unwrap_or("").to_string(),
        push_url: remote.pushurl().map(String::from),
        sync_status: SyncStatus::Unknown,
    })
}

/// Get ahead/behind counts for the current branch vs its upstream.
///
/// Returns `(ahead, behind)` as a tuple for compatibility with callers that
/// destructure directly.
pub fn get_ahead_behind(repo: &Repository) -> Result<(usize, usize)> {
    let head = match repo.head() {
        Ok(h) => h,
        Err(_) => return Ok((0, 0)),
    };

    let local_oid = match head.target() {
        Some(oid) => oid,
        None => return Ok((0, 0)),
    };

    // Find the upstream branch.
    let branch_name = match head.shorthand() {
        Some(name) => name.to_string(),
        None => return Ok((0, 0)),
    };

    let branch = match repo.find_branch(&branch_name, git2::BranchType::Local) {
        Ok(b) => b,
        Err(_) => return Ok((0, 0)),
    };

    let upstream = match branch.upstream() {
        Ok(u) => u,
        Err(_) => return Ok((0, 0)),
    };

    let upstream_oid = match upstream.get().target() {
        Some(oid) => oid,
        None => return Ok((0, 0)),
    };

    let (ahead, behind) = repo
        .graph_ahead_behind(local_oid, upstream_oid)
        .context("failed to compute ahead/behind")?;
    Ok((ahead, behind))
}

/// Compute `SyncStatus` for a named branch.
pub fn sync_status(repo: &Repository, branch_name: &str) -> Result<SyncStatus> {
    let local = match repo.find_branch(branch_name, git2::BranchType::Local) {
        Ok(b) => b,
        Err(_) => return Ok(SyncStatus::default()),
    };

    let upstream = match local.upstream() {
        Ok(u) => u,
        Err(_) => return Ok(SyncStatus::default()),
    };

    let local_oid = match local.get().peel_to_commit() {
        Ok(c) => c.id(),
        Err(_) => return Ok(SyncStatus::default()),
    };

    let upstream_oid = match upstream.get().peel_to_commit() {
        Ok(c) => c.id(),
        Err(_) => return Ok(SyncStatus::default()),
    };

    let (ahead, behind) = repo
        .graph_ahead_behind(local_oid, upstream_oid)
        .unwrap_or((0, 0));

    Ok(SyncStatus::from_counts(ahead, behind))
}

/// Fetch all refs from a remote using `git2`.
///
/// Authentication is handled via the default credential helpers configured in
/// the user's git configuration.
pub fn fetch(repo: &Repository, remote_name: &str) -> Result<()> {
    let mut remote = repo
        .find_remote(remote_name)
        .with_context(|| format!("remote '{remote_name}' not found"))?;

    let mut callbacks = git2::RemoteCallbacks::new();

    callbacks.credentials(|url, username_from_url, allowed_types| {
        // Try SSH agent first.
        if allowed_types.contains(git2::CredentialType::SSH_KEY) {
            if let Some(user) = username_from_url {
                return git2::Cred::ssh_key_from_agent(user);
            }
        }

        // Try credential helper.
        if allowed_types.contains(git2::CredentialType::USER_PASS_PLAINTEXT) {
            return git2::Cred::credential_helper(
                &repo.config()
                    .unwrap_or_else(|_| git2::Config::open_default().unwrap()),
                url,
                username_from_url,
            );
        }

        // Default credentials.
        if allowed_types.contains(git2::CredentialType::DEFAULT) {
            return git2::Cred::default();
        }

        Err(git2::Error::from_str("no suitable credentials found"))
    });

    let mut fetch_opts = git2::FetchOptions::new();
    fetch_opts.remote_callbacks(callbacks);
    fetch_opts.download_tags(git2::AutotagOption::Auto);

    remote
        .fetch(&[] as &[&str], Some(&mut fetch_opts), None)
        .with_context(|| format!("failed to fetch from '{remote_name}'"))?;

    Ok(())
}

/// Push the current branch to the specified remote.
///
/// Falls back to the system `git` binary because `git2` push has complicated
/// auth handling.
pub fn push(repo: &Repository, remote_name: &str, branch_name: &str) -> Result<()> {
    let workdir = repo
        .workdir()
        .context("repository is bare")?;

    let output = Command::new("git")
        .arg("push")
        .arg(remote_name)
        .arg(branch_name)
        .current_dir(workdir)
        .output()
        .context("failed to execute git push")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git push failed: {stderr}");
    }

    Ok(())
}

/// Push with `--set-upstream` flag.
pub fn push_set_upstream(
    repo: &Repository,
    remote_name: &str,
    branch_name: &str,
) -> Result<()> {
    let workdir = repo
        .workdir()
        .context("repository is bare")?;

    let output = Command::new("git")
        .arg("push")
        .arg("--set-upstream")
        .arg(remote_name)
        .arg(branch_name)
        .current_dir(workdir)
        .output()
        .context("failed to execute git push --set-upstream")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git push --set-upstream failed: {stderr}");
    }

    Ok(())
}

/// Pull (fetch + merge) via the system `git` binary.
pub fn pull(repo: &Repository, remote_name: &str, branch_name: &str) -> Result<()> {
    let workdir = repo
        .workdir()
        .context("repository is bare")?;

    let output = Command::new("git")
        .arg("pull")
        .arg(remote_name)
        .arg(branch_name)
        .current_dir(workdir)
        .output()
        .context("failed to execute git pull")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git pull failed: {stderr}");
    }

    Ok(())
}

/// Pull with rebase via the system `git` binary.
pub fn pull_rebase(repo: &Repository, remote_name: &str, branch_name: &str) -> Result<()> {
    let workdir = repo
        .workdir()
        .context("repository is bare")?;

    let output = Command::new("git")
        .arg("pull")
        .arg("--rebase")
        .arg(remote_name)
        .arg(branch_name)
        .current_dir(workdir)
        .output()
        .context("failed to execute git pull --rebase")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git pull --rebase failed: {stderr}");
    }

    Ok(())
}

/// Force-push with lease via the system `git` binary.
pub fn force_push_with_lease(
    repo: &Repository,
    remote_name: &str,
    branch_name: &str,
) -> Result<()> {
    let workdir = repo
        .workdir()
        .context("repository is bare")?;

    let output = Command::new("git")
        .arg("push")
        .arg("--force-with-lease")
        .arg(remote_name)
        .arg(branch_name)
        .current_dir(workdir)
        .output()
        .context("failed to execute git push --force-with-lease")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git push --force-with-lease failed: {stderr}");
    }

    Ok(())
}
