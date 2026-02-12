use anyhow::{bail, Context, Result};
use git2::Repository;

/// Information about a single stash entry.
#[derive(Debug, Clone)]
pub struct StashInfo {
    /// Stash index (0 is the most recent).
    pub index: usize,
    /// Stash message.
    pub message: String,
    /// The OID of the stash commit (as a hex string).
    pub oid: String,
}

/// List all stash entries (most recent first, matching `git stash list`
/// ordering).
pub fn list_stashes(repo: &mut Repository) -> Result<Vec<StashInfo>> {
    let mut stashes = Vec::new();

    repo.stash_foreach(|index, message, oid| {
        stashes.push(StashInfo {
            index,
            message: message.to_string(),
            oid: oid.to_string(),
        });
        true // continue iterating
    })
    .context("failed to enumerate stash entries")?;

    Ok(stashes)
}

/// Save a new stash with an optional message.
///
/// If `message` is `None`, a default message is used.
pub fn stash_save(repo: &mut Repository, message: Option<&str>) -> Result<git2::Oid> {
    let sig = repo
        .signature()
        .context("could not determine stash author; set user.name and user.email")?;

    let msg = message.unwrap_or("WIP on gittui stash");
    let oid = repo
        .stash_save(&sig, msg, Some(git2::StashFlags::DEFAULT))
        .context("failed to save stash (is the working tree clean?)")?;

    Ok(oid)
}

/// Save a stash including untracked files.
pub fn stash_save_untracked(repo: &mut Repository, message: Option<&str>) -> Result<git2::Oid> {
    let sig = repo
        .signature()
        .context("could not determine stash author")?;

    let flags = git2::StashFlags::DEFAULT | git2::StashFlags::INCLUDE_UNTRACKED;
    let msg = message.unwrap_or("WIP on gittui stash (with untracked)");
    let oid = repo
        .stash_save(&sig, msg, Some(flags))
        .context("failed to save stash with untracked files")?;

    Ok(oid)
}

/// Save a stash and keep the index staged state intact (like
/// `git stash --keep-index`).
pub fn stash_save_keep_index(repo: &mut Repository, message: Option<&str>) -> Result<git2::Oid> {
    let sig = repo
        .signature()
        .context("could not determine stash author")?;

    let flags = git2::StashFlags::KEEP_INDEX;
    let msg = message.unwrap_or("WIP on gittui stash (keep index)");
    let oid = repo
        .stash_save(&sig, msg, Some(flags))
        .context("failed to save stash with --keep-index")?;

    Ok(oid)
}

/// Pop the stash at the given index (default: 0, the most recent).
///
/// Applies the stash and removes it from the stash list.
pub fn stash_pop(repo: &mut Repository) -> Result<()> {
    repo.stash_pop(0, None)
        .context("failed to pop stash@{0}")?;
    Ok(())
}

/// Pop a specific stash by index.
pub fn stash_pop_index(repo: &mut Repository, index: usize) -> Result<()> {
    let mut opts = git2::StashApplyOptions::new();
    opts.progress_cb(|_progress| true);
    repo.stash_pop(index, Some(&mut opts))
        .with_context(|| format!("failed to pop stash@{{{index}}}"))?;
    Ok(())
}

/// Apply a specific stash by index (without dropping).
pub fn stash_apply(repo: &mut Repository, index: usize) -> Result<()> {
    repo.stash_apply(index, None)
        .with_context(|| format!("failed to apply stash@{{{index}}}"))?;
    Ok(())
}

/// Drop a specific stash by index.
pub fn stash_drop(repo: &mut Repository, index: usize) -> Result<()> {
    repo.stash_drop(index)
        .with_context(|| format!("failed to drop stash@{{{index}}}"))?;
    Ok(())
}

/// Drop all stash entries.
pub fn stash_clear(repo: &mut Repository) -> Result<()> {
    let entries = list_stashes(repo)?;
    if entries.is_empty() {
        bail!("no stash entries to clear");
    }
    // Drop from the end to avoid index shifting problems.
    for entry in entries.iter().rev() {
        repo.stash_drop(entry.index)
            .with_context(|| format!("failed to drop stash@{{{}}}", entry.index))?;
    }
    Ok(())
}
