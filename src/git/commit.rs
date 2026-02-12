use anyhow::{bail, Context, Result};
use git2::{Repository, Signature};

/// Create a new commit from the current index (staged changes).
///
/// Returns the OID of the newly created commit.
pub fn commit(repo: &Repository, message: &str) -> Result<git2::Oid> {
    if message.trim().is_empty() {
        bail!("commit message must not be empty");
    }

    let sig = repo
        .signature()
        .context("could not create signature; set user.name and user.email")?;
    let mut index = repo.index().context("failed to open index")?;
    let tree_oid = index.write_tree().context("failed to write index tree")?;
    let tree = repo
        .find_tree(tree_oid)
        .context("failed to find tree for index")?;

    let parent = match repo.head() {
        Ok(head) => Some(head.peel_to_commit()?),
        Err(_) => None, // initial commit
    };

    let parents: Vec<&git2::Commit<'_>> = parent.iter().collect();

    let oid = repo
        .commit(Some("HEAD"), &sig, &sig, message, &tree, &parents)
        .context("failed to create commit")?;

    Ok(oid)
}

/// Amend the most recent commit with a new message and/or the current index
/// state.
pub fn amend(repo: &Repository, message: &str) -> Result<git2::Oid> {
    if message.trim().is_empty() {
        bail!("commit message must not be empty");
    }

    let head = repo.head().context("no HEAD to amend")?;
    let head_commit = head
        .peel_to_commit()
        .context("HEAD does not point to a commit")?;

    let sig = repo
        .signature()
        .context("could not create signature; set user.name and user.email")?;

    // Build tree from the current index so that any staged changes are
    // included in the amended commit.
    let mut index = repo.index().context("failed to open index")?;
    let tree_oid = index.write_tree().context("failed to write index tree")?;
    let tree = repo
        .find_tree(tree_oid)
        .context("failed to find tree for index")?;

    let oid = head_commit
        .amend(
            Some("HEAD"),
            Some(&sig),
            Some(&sig),
            None, // encoding
            Some(message),
            Some(&tree),
        )
        .context("failed to amend commit")?;

    Ok(oid)
}

/// Amend with an optional new message. If `None`, the original message is
/// kept.
pub fn amend_optional_message(
    repo: &Repository,
    new_message: Option<&str>,
) -> Result<git2::Oid> {
    let head = repo.head().context("no HEAD to amend")?;
    let head_commit = head
        .peel_to_commit()
        .context("HEAD does not point to a commit")?;

    let message = match new_message {
        Some(m) => m.to_string(),
        None => head_commit
            .message()
            .unwrap_or("")
            .to_string(),
    };

    amend(repo, &message)
}

/// Create a commit with a specific signature (useful for scripted/test
/// scenarios).
pub fn commit_with_signature(
    repo: &Repository,
    message: &str,
    author: &Signature<'_>,
    committer: &Signature<'_>,
) -> Result<git2::Oid> {
    if message.trim().is_empty() {
        bail!("commit message must not be empty");
    }

    let mut index = repo.index().context("failed to open index")?;
    let tree_oid = index.write_tree().context("failed to write index tree")?;
    let tree = repo
        .find_tree(tree_oid)
        .context("failed to find tree for index")?;

    let parent = match repo.head() {
        Ok(head) => Some(head.peel_to_commit()?),
        Err(_) => None,
    };
    let parents: Vec<&git2::Commit<'_>> = parent.iter().collect();

    let oid = repo
        .commit(Some("HEAD"), author, committer, message, &tree, &parents)
        .context("failed to create commit")?;

    Ok(oid)
}

/// Create a merge commit with multiple parents.
pub fn merge_commit(
    repo: &Repository,
    message: &str,
    parents: &[git2::Oid],
) -> Result<git2::Oid> {
    if message.trim().is_empty() {
        bail!("commit message must not be empty");
    }

    let sig = repo
        .signature()
        .context("could not create signature")?;

    let mut index = repo.index().context("failed to open index")?;
    let tree_oid = index.write_tree().context("failed to write index tree")?;
    let tree = repo
        .find_tree(tree_oid)
        .context("failed to find tree for index")?;

    let parent_commits: Vec<git2::Commit<'_>> = parents
        .iter()
        .map(|oid| {
            repo.find_commit(*oid)
                .with_context(|| format!("parent commit {oid} not found"))
        })
        .collect::<Result<Vec<_>>>()?;

    let parent_refs: Vec<&git2::Commit<'_>> = parent_commits.iter().collect();

    let oid = repo
        .commit(Some("HEAD"), &sig, &sig, message, &tree, &parent_refs)
        .context("failed to create merge commit")?;

    Ok(oid)
}
