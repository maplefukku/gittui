use anyhow::{Context, Result};
use git2::{Diff, DiffDelta, DiffFormat, DiffHunk, DiffLine as Git2DiffLine, DiffOptions, Repository};

/// Type of a diff line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffLineType {
    Context,
    Addition,
    Deletion,
    HunkHeader,
    Binary,
}

/// A single line in a diff.
#[derive(Debug, Clone)]
pub struct DiffLine {
    pub line_type: DiffLineType,
    pub old_lineno: Option<u32>,
    pub new_lineno: Option<u32>,
    pub content: String,
}

/// A single hunk in a diff.
#[derive(Debug, Clone)]
pub struct HunkInfo {
    pub header: String,
    pub old_start: u32,
    pub old_lines: u32,
    pub new_start: u32,
    pub new_lines: u32,
    pub lines: Vec<DiffLine>,
}

/// The full diff for a single file.
#[derive(Debug, Clone)]
pub struct FileDiff {
    pub path: String,
    pub old_path: Option<String>,
    pub hunks: Vec<HunkInfo>,
    pub is_binary: bool,
    /// Total additions in this diff.
    pub additions: usize,
    /// Total deletions in this diff.
    pub deletions: usize,
}

/// Where to diff from: index-to-workdir (unstaged) or HEAD-to-index (staged).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffTarget {
    /// Changes in the working directory not yet staged.
    Unstaged,
    /// Changes that have been staged (index vs HEAD).
    Staged,
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Build a `git2::Diff` for the requested target, optionally filtered to a
/// single path.
fn build_diff<'a>(repo: &'a Repository, target: DiffTarget, path: Option<&'a str>) -> Result<Diff<'a>> {
    let mut opts = DiffOptions::new();
    opts.include_untracked(true)
        .recurse_untracked_dirs(true)
        .show_untracked_content(true)
        .context_lines(3);

    if let Some(p) = path {
        opts.pathspec(p);
    }

    match target {
        DiffTarget::Unstaged => repo
            .diff_index_to_workdir(None, Some(&mut opts))
            .context("failed to compute unstaged diff"),
        DiffTarget::Staged => {
            let head_tree = repo
                .head()
                .ok()
                .and_then(|h| h.peel_to_tree().ok());
            repo.diff_tree_to_index(head_tree.as_ref(), None, Some(&mut opts))
                .context("failed to compute staged diff")
        }
    }
}

/// Parse a `git2::Diff` into a `FileDiff` for a specific path.
///
/// Uses `diff.print` with a single callback to avoid the multiple-mutable-
/// borrow issues that arise with `diff.foreach`.
fn parse_diff(diff: &Diff<'_>, path: &str) -> Result<FileDiff> {
    let mut hunks: Vec<HunkInfo> = Vec::new();
    let mut is_binary = false;
    let mut additions: usize = 0;
    let mut deletions: usize = 0;
    let mut old_path: Option<String> = None;

    diff.print(DiffFormat::Patch, |delta, hunk_opt, line| {
        // Check for binary flag on the delta.
        if delta.flags().is_binary() {
            is_binary = true;
        }

        // Capture old path for renames (only once).
        if old_path.is_none() {
            if let Some(op) = delta.old_file().path() {
                let op_str = op.to_string_lossy().to_string();
                if op_str != path {
                    old_path = Some(op_str);
                }
            }
        }

        // File-level header lines ('F') -- skip.
        if line.origin() == 'F' {
            return true;
        }

        // Hunk header line ('H') -- create a new hunk.
        if line.origin() == 'H' {
            if let Some(h) = hunk_opt {
                let header = String::from_utf8_lossy(h.header()).trim().to_string();
                hunks.push(HunkInfo {
                    header,
                    old_start: h.old_start(),
                    old_lines: h.old_lines(),
                    new_start: h.new_start(),
                    new_lines: h.new_lines(),
                    lines: Vec::new(),
                });
            }
            return true;
        }

        // Content lines -- add to the current hunk.
        let content = String::from_utf8_lossy(line.content()).to_string();
        let line_type = match line.origin() {
            '+' => {
                additions += 1;
                DiffLineType::Addition
            }
            '-' => {
                deletions += 1;
                DiffLineType::Deletion
            }
            'B' => DiffLineType::Binary,
            _ => DiffLineType::Context,
        };

        if let Some(hunk_entry) = hunks.last_mut() {
            hunk_entry.lines.push(DiffLine {
                line_type,
                old_lineno: line.old_lineno(),
                new_lineno: line.new_lineno(),
                content,
            });
        }

        true
    })
    .context("error while iterating diff")?;

    Ok(FileDiff {
        path: path.to_string(),
        old_path,
        hunks,
        is_binary,
        additions,
        deletions,
    })
}

/// Walk a `git2::Diff` and collect `FileDiff` entries for all files.
fn collect_file_diffs(diff: &Diff<'_>) -> Result<Vec<FileDiff>> {
    let mut files: Vec<FileDiff> = Vec::new();

    // Use diff.print which provides a single callback that receives all
    // information, avoiding the multiple-mutable-borrow issue of foreach.
    diff.print(DiffFormat::Patch, |delta, hunk, line| {
        let path = delta
            .new_file()
            .path()
            .or_else(|| delta.old_file().path())
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default();

        // Ensure we have a FileDiff entry for this file.
        let needs_new_file = files.last().map(|f| f.path != path).unwrap_or(true);
        if needs_new_file {
            let old_path = delta.old_file().path().and_then(|p| {
                let s = p.to_string_lossy().into_owned();
                if s == path { None } else { Some(s) }
            });
            files.push(FileDiff {
                path: path.clone(),
                old_path,
                hunks: Vec::new(),
                is_binary: delta.flags().is_binary(),
                additions: 0,
                deletions: 0,
            });
        }

        let file = files.last_mut().unwrap();

        // If we have a hunk header, ensure we have a HunkInfo for it.
        if let Some(ref h) = hunk {
            let hunk_header = String::from_utf8_lossy(h.header()).trim_end().to_string();
            let needs_new_hunk = file
                .hunks
                .last()
                .map(|hk| hk.header != hunk_header)
                .unwrap_or(true);
            if needs_new_hunk {
                file.hunks.push(HunkInfo {
                    header: hunk_header,
                    old_start: h.old_start(),
                    old_lines: h.old_lines(),
                    new_start: h.new_start(),
                    new_lines: h.new_lines(),
                    lines: Vec::new(),
                });
            }
        }

        // Add the line to the current hunk.
        let line_type = match line.origin() {
            '+' => DiffLineType::Addition,
            '-' => DiffLineType::Deletion,
            'H' | 'F' => DiffLineType::HunkHeader,
            'B' => DiffLineType::Binary,
            _ => DiffLineType::Context,
        };

        match line_type {
            DiffLineType::Addition => file.additions += 1,
            DiffLineType::Deletion => file.deletions += 1,
            _ => {}
        }

        // Only add actual content lines (not file-level headers).
        if hunk.is_some() {
            let content = String::from_utf8_lossy(line.content()).to_string();
            if let Some(hunk_entry) = file.hunks.last_mut() {
                hunk_entry.lines.push(DiffLine {
                    line_type,
                    old_lineno: line.old_lineno(),
                    new_lineno: line.new_lineno(),
                    content,
                });
            }
        }

        true
    })
    .context("error while iterating diff")?;

    Ok(files)
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Get the diff for a staged file (HEAD vs index).
pub fn get_staged_diff(repo: &Repository, path: &str) -> Result<FileDiff> {
    let diff = build_diff(repo, DiffTarget::Staged, Some(path))?;
    parse_diff(&diff, path)
}

/// Get the diff for an unstaged file (index vs workdir).
pub fn get_unstaged_diff(repo: &Repository, path: &str) -> Result<FileDiff> {
    let diff = build_diff(repo, DiffTarget::Unstaged, Some(path))?;
    parse_diff(&diff, path)
}

/// Return diffs for all files matching `target`.
pub fn get_diff(repo: &Repository, target: DiffTarget) -> Result<Vec<FileDiff>> {
    let diff = build_diff(repo, target, None)?;
    collect_file_diffs(&diff)
}

/// Return the diff for a single file given a target.
pub fn get_file_diff(
    repo: &Repository,
    path: &str,
    target: DiffTarget,
) -> Result<Option<FileDiff>> {
    let diff = build_diff(repo, target, Some(path))?;
    let files = collect_file_diffs(&diff)?;
    Ok(files.into_iter().find(|f| f.path == path))
}

/// Generate a unified diff string (like `git diff`) for a single file.
pub fn unified_diff_string(
    repo: &Repository,
    path: &str,
    target: DiffTarget,
) -> Result<String> {
    let diff = build_diff(repo, target, Some(path))?;
    let mut buf = Vec::new();
    diff.print(DiffFormat::Patch, |_delta: DiffDelta<'_>, _hunk: Option<DiffHunk<'_>>, line: Git2DiffLine<'_>| {
        match line.origin() {
            '+' | '-' | ' ' => buf.push(line.origin() as u8),
            _ => {}
        }
        buf.extend_from_slice(line.content());
        true
    })
    .context("failed to print diff")?;
    Ok(String::from_utf8_lossy(&buf).into_owned())
}

/// Compute diff stats (files changed, insertions, deletions).
pub fn diff_stats(repo: &Repository, target: DiffTarget) -> Result<(usize, usize, usize)> {
    let diff = build_diff(repo, target, None)?;
    let stats = diff.stats().context("failed to compute diff stats")?;
    Ok((
        stats.files_changed(),
        stats.insertions(),
        stats.deletions(),
    ))
}

/// Diff between two arbitrary commits, optionally filtered to a single path.
pub fn diff_commits(
    repo: &Repository,
    old_oid: git2::Oid,
    new_oid: git2::Oid,
    path: Option<&str>,
) -> Result<Vec<FileDiff>> {
    let old_tree = repo
        .find_commit(old_oid)
        .context("old commit not found")?
        .tree()
        .context("old commit has no tree")?;
    let new_tree = repo
        .find_commit(new_oid)
        .context("new commit not found")?
        .tree()
        .context("new commit has no tree")?;

    let mut opts = DiffOptions::new();
    opts.context_lines(3);
    if let Some(p) = path {
        opts.pathspec(p);
    }

    let diff = repo
        .diff_tree_to_tree(Some(&old_tree), Some(&new_tree), Some(&mut opts))
        .context("failed to diff commits")?;

    collect_file_diffs(&diff)
}

// ---------------------------------------------------------------------------
// Hunk patch generation and application
// ---------------------------------------------------------------------------

/// Generate a patch string for a single hunk of a file diff.
pub fn generate_hunk_patch(diff: &FileDiff, hunk_index: usize) -> Option<String> {
    let hunk = diff.hunks.get(hunk_index)?;
    let mut patch = String::new();

    // Minimal diff header
    patch.push_str(&format!("--- a/{}\n", diff.old_path.as_deref().unwrap_or(&diff.path)));
    patch.push_str(&format!("+++ b/{}\n", diff.path));
    patch.push_str(&format!("{}\n", hunk.header));

    for line in &hunk.lines {
        match line.line_type {
            DiffLineType::Addition => {
                patch.push('+');
                patch.push_str(&line.content);
                if !line.content.ends_with('\n') {
                    patch.push('\n');
                }
            }
            DiffLineType::Deletion => {
                patch.push('-');
                patch.push_str(&line.content);
                if !line.content.ends_with('\n') {
                    patch.push('\n');
                }
            }
            DiffLineType::Context => {
                patch.push(' ');
                patch.push_str(&line.content);
                if !line.content.ends_with('\n') {
                    patch.push('\n');
                }
            }
            _ => {}
        }
    }

    Some(patch)
}

/// Stage a single hunk by applying a patch to the index.
pub fn stage_hunk(repo_path: &std::path::Path, patch: &str) -> Result<()> {
    use std::io::Write;
    let mut child = std::process::Command::new("git")
        .args(["apply", "--cached", "--unidiff-zero"])
        .current_dir(repo_path)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context("failed to spawn git apply")?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(patch.as_bytes()).context("failed to write patch to stdin")?;
    }

    let output = child.wait_with_output().context("failed to wait for git apply")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git apply --cached failed: {stderr}");
    }

    Ok(())
}

/// Discard a single hunk by reverse-applying a patch.
pub fn discard_hunk(repo_path: &std::path::Path, patch: &str) -> Result<()> {
    use std::io::Write;
    let mut child = std::process::Command::new("git")
        .args(["apply", "--reverse"])
        .current_dir(repo_path)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context("failed to spawn git apply --reverse")?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(patch.as_bytes()).context("failed to write patch to stdin")?;
    }

    let output = child.wait_with_output().context("failed to wait for git apply --reverse")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git apply --reverse failed: {stderr}");
    }

    Ok(())
}
