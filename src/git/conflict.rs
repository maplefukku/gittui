use anyhow::{Context, Result};
use git2::{IndexConflict, Repository};
use std::path::Path;

/// Which side of the conflict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictSide {
    Ancestor,
    Ours,
    Theirs,
}

/// A single conflict entry from the index.
#[derive(Debug, Clone)]
pub struct ConflictEntry {
    /// The file path.
    pub path: String,
    /// Which sides are present.
    pub sides: Vec<ConflictSide>,
    /// OID of the ancestor blob (if present).
    pub ancestor_oid: Option<git2::Oid>,
    /// OID of our blob (if present).
    pub our_oid: Option<git2::Oid>,
    /// OID of their blob (if present).
    pub their_oid: Option<git2::Oid>,
}

/// Parsed conflict markers from a file on disk.
#[derive(Debug, Clone)]
pub struct ConflictRegion {
    /// Line number (1-based) where the conflict starts (`<<<<<<<`).
    pub start_line: usize,
    /// Line number (1-based) where the conflict ends (`>>>>>>>`).
    pub end_line: usize,
    /// Lines belonging to "ours" (between `<<<<<<<` and `=======`).
    pub ours: Vec<String>,
    /// Lines belonging to "theirs" (between `=======` and `>>>>>>>`).
    pub theirs: Vec<String>,
}

/// How to resolve a conflict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictResolution {
    ChooseOurs,
    ChooseTheirs,
}

/// List all conflicted files in the repository.
pub fn list_conflicts(repo: &Repository) -> Result<Vec<ConflictEntry>> {
    let index = repo.index().context("failed to open index")?;
    let conflicts = index.conflicts().context("failed to read conflicts")?;
    let mut entries = Vec::new();

    for conflict in conflicts {
        let IndexConflict {
            ancestor,
            our,
            their,
        } = conflict.context("error reading conflict entry")?;

        let path = our
            .as_ref()
            .or(their.as_ref())
            .or(ancestor.as_ref())
            .and_then(|e| String::from_utf8(e.path.clone()).ok())
            .unwrap_or_else(|| "<unknown>".to_string());

        let mut sides = Vec::new();
        if ancestor.is_some() {
            sides.push(ConflictSide::Ancestor);
        }
        if our.is_some() {
            sides.push(ConflictSide::Ours);
        }
        if their.is_some() {
            sides.push(ConflictSide::Theirs);
        }

        entries.push(ConflictEntry {
            path,
            sides,
            ancestor_oid: ancestor.map(|e| e.id),
            our_oid: our.map(|e| e.id),
            their_oid: their.map(|e| e.id),
        });
    }

    Ok(entries)
}

/// Return `true` if the repository currently has unresolved merge conflicts.
pub fn has_conflicts(repo: &Repository) -> Result<bool> {
    let index = repo.index().context("failed to open index")?;
    Ok(index.has_conflicts())
}

/// Read the blob content for one side of a conflict.
pub fn read_conflict_side(
    repo: &Repository,
    entry: &ConflictEntry,
    side: ConflictSide,
) -> Result<Option<String>> {
    let oid = match side {
        ConflictSide::Ancestor => entry.ancestor_oid,
        ConflictSide::Ours => entry.our_oid,
        ConflictSide::Theirs => entry.their_oid,
    };

    match oid {
        Some(id) => {
            let blob = repo
                .find_blob(id)
                .with_context(|| format!("blob {id} not found"))?;
            let content = String::from_utf8_lossy(blob.content()).into_owned();
            Ok(Some(content))
        }
        None => Ok(None),
    }
}

/// Parse conflict markers in a file on disk and return the conflict regions.
///
/// Handles the standard three-way merge markers:
/// ```text
/// <<<<<<< HEAD
/// our lines ...
/// =======
/// their lines ...
/// >>>>>>> branch
/// ```
pub fn parse_conflict_markers(file_path: &Path) -> Result<Vec<ConflictRegion>> {
    let content = std::fs::read_to_string(file_path)
        .with_context(|| format!("failed to read {}", file_path.display()))?;
    parse_conflict_markers_from_string(&content)
}

/// Parse conflict markers from a string (useful for testing or when the
/// content is already in memory).
pub fn parse_conflict_markers_from_string(content: &str) -> Result<Vec<ConflictRegion>> {
    let mut regions = Vec::new();

    let mut in_conflict = false;
    let mut in_ours = false;
    let mut start_line: usize = 0;
    let mut ours: Vec<String> = Vec::new();
    let mut theirs: Vec<String> = Vec::new();

    for (i, line) in content.lines().enumerate() {
        let lineno = i + 1; // 1-based

        if line.starts_with("<<<<<<<") {
            in_conflict = true;
            in_ours = true;
            start_line = lineno;
            ours.clear();
            theirs.clear();
        } else if line.starts_with("=======") && in_conflict {
            in_ours = false;
        } else if line.starts_with(">>>>>>>") && in_conflict {
            regions.push(ConflictRegion {
                start_line,
                end_line: lineno,
                ours: ours.clone(),
                theirs: theirs.clone(),
            });
            in_conflict = false;
            in_ours = false;
        } else if in_conflict {
            if in_ours {
                ours.push(line.to_string());
            } else {
                theirs.push(line.to_string());
            }
        }
    }

    Ok(regions)
}

/// Resolve a conflict by choosing one side and staging the result.
pub fn resolve_conflict(
    repo: &Repository,
    entry: &ConflictEntry,
    resolution: ConflictResolution,
) -> Result<()> {
    let oid = match resolution {
        ConflictResolution::ChooseOurs => entry
            .our_oid
            .context("our side does not exist for this conflict")?,
        ConflictResolution::ChooseTheirs => entry
            .their_oid
            .context("their side does not exist for this conflict")?,
    };

    let blob = repo.find_blob(oid).context("conflict blob not found")?;
    let workdir = repo.workdir().context("repository is bare")?;
    let file_path = workdir.join(&entry.path);

    // Ensure parent directories exist.
    if let Some(parent) = file_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create directories for {}", entry.path))?;
    }

    std::fs::write(&file_path, blob.content())
        .with_context(|| format!("failed to write resolved file {}", entry.path))?;

    // Stage the resolved file to mark the conflict as resolved.
    let mut index = repo.index().context("failed to open index")?;
    index
        .add_path(Path::new(&entry.path))
        .with_context(|| format!("failed to stage resolved file {}", entry.path))?;
    index.write().context("failed to write index")?;

    Ok(())
}

/// Resolve a conflict by writing custom content and staging it.
pub fn resolve_conflict_with_content(
    repo: &Repository,
    path: &str,
    content: &[u8],
) -> Result<()> {
    let workdir = repo.workdir().context("repository is bare")?;
    let file_path = workdir.join(path);

    if let Some(parent) = file_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    std::fs::write(&file_path, content)
        .with_context(|| format!("failed to write resolved file {path}"))?;

    let mut index = repo.index().context("failed to open index")?;
    index
        .add_path(Path::new(path))
        .with_context(|| format!("failed to stage resolved file {path}"))?;
    index.write().context("failed to write index")?;

    Ok(())
}

/// Return `true` if the repository is in a merge state.
pub fn is_merging(repo: &Repository) -> bool {
    repo.state() == git2::RepositoryState::Merge
}

/// Return `true` if the repository is in a rebase state.
pub fn is_rebasing(repo: &Repository) -> bool {
    matches!(
        repo.state(),
        git2::RepositoryState::Rebase
            | git2::RepositoryState::RebaseInteractive
            | git2::RepositoryState::RebaseMerge
    )
}

/// Return `true` if the repository is in a cherry-pick state.
pub fn is_cherry_picking(repo: &Repository) -> bool {
    repo.state() == git2::RepositoryState::CherryPick
}

/// Abort the current merge (equivalent to `git merge --abort`).
pub fn abort_merge(repo: &Repository) -> Result<()> {
    repo.cleanup_state().context("failed to abort merge")?;

    let head = repo.head().context("no HEAD")?;
    let obj = head
        .peel(git2::ObjectType::Commit)
        .context("HEAD does not point to a commit")?;

    repo.reset(&obj, git2::ResetType::Hard, None)
        .context("failed to reset to HEAD after merge abort")?;

    Ok(())
}
