use anyhow::{Context, Result};
use git2::{Oid, Repository, Sort};
use std::collections::HashMap;

/// Symbol used in the graph column for log display.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphSymbol {
    /// The commit dot itself (`*`).
    Commit,
    /// A merge line coming from the right (`\`).
    Merge,
    /// A vertical pipe connecting parent/child (`|`).
    Line,
    /// Alias for `Line` used by UI.
    Vertical,
    /// A branch line going to the right (`/`).
    Branch,
    /// Branch off (T-junction).
    BranchOff,
    /// Horizontal connector.
    Horizontal,
    /// Merge top (top-left curve).
    MergeTop,
    /// Merge bottom (bottom-left curve).
    MergeBottom,
    /// Empty space.
    Empty,
    /// Alias for `Empty` used by UI.
    Space,
}

impl GraphSymbol {
    /// Render the symbol as a single character for display.
    pub fn as_char(&self) -> char {
        match self {
            GraphSymbol::Commit => '*',
            GraphSymbol::Line | GraphSymbol::Vertical => '|',
            GraphSymbol::Merge | GraphSymbol::MergeTop | GraphSymbol::MergeBottom => '\\',
            GraphSymbol::Branch | GraphSymbol::BranchOff => '/',
            GraphSymbol::Horizontal => '-',
            GraphSymbol::Empty | GraphSymbol::Space => ' ',
        }
    }
}

/// Information about a single commit in the log.
#[derive(Debug, Clone)]
pub struct CommitInfo {
    /// Full OID hex string.
    pub oid: String,
    /// Short OID (first 7 chars).
    pub short_oid: String,
    /// Alias for `short_oid` used by some UI code.
    pub short_hash: String,
    /// Commit summary (first line).
    pub summary: String,
    /// Full commit message.
    pub message: String,
    /// Author name.
    pub author: String,
    /// Author email.
    pub author_email: String,
    /// Commit timestamp as a formatted string.
    pub date: String,
    /// Relative time like "2 hours ago".
    pub relative_date: String,
    /// Refs (branches/tags) pointing at this commit.
    pub refs: Vec<String>,
    /// Graph symbols for this row, one per lane (column).
    pub graph: Vec<GraphSymbol>,
    /// Alias for `graph` used by some UI code.
    pub graph_symbols: Vec<GraphSymbol>,
    /// Number of parents (>1 means merge).
    pub parent_count: usize,
    /// Parent OID hex strings.
    pub parents: Vec<String>,
    /// Whether this commit is at HEAD.
    pub is_head: bool,
}

/// Load commit log with graph lane computation.
///
/// Walks commits from HEAD using topological + time ordering and computes
/// which lane (column) each commit occupies in the ASCII graph.
pub fn get_log(repo: &Repository, max_count: usize) -> Result<Vec<CommitInfo>> {
    get_log_from(repo, None, max_count)
}

/// Load commit log starting from a specific OID (or HEAD when `start` is
/// `None`).
pub fn get_log_from(
    repo: &Repository,
    start: Option<Oid>,
    max_count: usize,
) -> Result<Vec<CommitInfo>> {
    let mut revwalk = repo.revwalk().context("failed to create revwalk")?;
    revwalk.set_sorting(Sort::TIME | Sort::TOPOLOGICAL)?;

    match start {
        Some(oid) => revwalk.push(oid)?,
        None => {
            if revwalk.push_head().is_err() {
                // Empty repository — return empty log.
                return Ok(Vec::new());
            }
        }
    }

    // Collect ref names for decoration.
    let ref_map = build_ref_map(repo)?;

    // Graph state: each entry is either Some(expected_oid) or None (empty lane).
    let mut lanes: Vec<Option<Oid>> = Vec::new();

    let mut commits = Vec::new();

    for oid_result in revwalk {
        if commits.len() >= max_count {
            break;
        }

        let oid = oid_result.context("error during revwalk")?;
        let commit = repo
            .find_commit(oid)
            .with_context(|| format!("commit {oid} not found during walk"))?;

        let parent_count = commit.parent_count();
        let parent_oids: Vec<Oid> = (0..parent_count)
            .filter_map(|i| commit.parent_id(i).ok())
            .collect();

        // --- Graph lane computation ---

        // 1. Find which lane this commit was expected in, or allocate a new one.
        let my_lane = find_or_allocate_lane(&mut lanes, oid);

        // 2. Build the graph row for this commit.
        let mut row = Vec::with_capacity(lanes.len());
        for (i, lane) in lanes.iter().enumerate() {
            if i == my_lane {
                row.push(GraphSymbol::Commit);
            } else if lane.is_some() {
                row.push(GraphSymbol::Line);
            } else {
                row.push(GraphSymbol::Empty);
            }
        }

        // 3. Route parent(s) into lanes.
        //    - First parent continues in `my_lane`.
        //    - Additional parents (merge) are placed in new or existing lanes.
        lanes[my_lane] = None;
        let mut extra_merge_lanes: Vec<usize> = Vec::new();

        for (pi, &parent_oid) in parent_oids.iter().enumerate() {
            if pi == 0 {
                lanes[my_lane] = Some(parent_oid);
            } else {
                let existing = lanes.iter().position(|l| *l == Some(parent_oid));
                match existing {
                    Some(idx) => {
                        extra_merge_lanes.push(idx);
                    }
                    None => {
                        let new_lane = allocate_lane(&mut lanes, parent_oid);
                        extra_merge_lanes.push(new_lane);
                    }
                }
            }
        }

        // 4. Extend the row to cover any newly created lanes, then draw
        //    merge/branch lines between my_lane and the extra parent lanes.
        while row.len() < lanes.len() {
            row.push(GraphSymbol::Empty);
        }
        for &ml in &extra_merge_lanes {
            if ml > my_lane {
                for col in (my_lane + 1)..=ml {
                    if row.get(col) == Some(&GraphSymbol::Empty) {
                        row[col] = GraphSymbol::Merge;
                    }
                }
            } else if ml < my_lane {
                for col in ml..my_lane {
                    if row.get(col) == Some(&GraphSymbol::Empty) {
                        row[col] = GraphSymbol::Branch;
                    }
                }
            }
        }

        // 5. Compact lanes: remove trailing None entries.
        while lanes.last() == Some(&None) {
            lanes.pop();
        }

        // --- Collect commit metadata ---
        let oid_str = oid.to_string();
        let short_oid = oid_str[..7.min(oid_str.len())].to_string();
        let summary = commit.summary().unwrap_or("").to_string();
        let message = commit.message().unwrap_or("").to_string();

        let author_sig = commit.author();
        let author = author_sig.name().unwrap_or("Unknown").to_string();
        let author_email = author_sig.email().unwrap_or("").to_string();

        let time = commit.time();
        let secs = time.seconds();
        let datetime = chrono::DateTime::from_timestamp(secs, 0).unwrap_or_default();
        let date = datetime.format("%Y-%m-%d %H:%M").to_string();

        let now = chrono::Utc::now();
        let duration = now.signed_duration_since(datetime);
        let relative_date = format_relative_time(duration);

        let refs = ref_map.get(&oid).cloned().unwrap_or_default();
        let parents: Vec<String> = parent_oids.iter().map(|o| o.to_string()).collect();

        let is_head = commits.is_empty(); // first commit in walk is HEAD
        commits.push(CommitInfo {
            oid: oid_str,
            short_hash: short_oid.clone(),
            short_oid,
            summary,
            message,
            author,
            author_email,
            date,
            relative_date,
            refs,
            graph_symbols: row.clone(),
            graph: row,
            parent_count,
            parents,
            is_head,
        });
    }

    Ok(commits)
}

/// Return the total number of commits reachable from HEAD.
pub fn commit_count(repo: &Repository) -> Result<usize> {
    let mut revwalk = repo.revwalk().context("failed to create revwalk")?;
    revwalk.push_head().context("no HEAD")?;
    Ok(revwalk.count())
}

/// Render a `Vec<GraphSymbol>` as a compact string for display.
pub fn render_graph_line(symbols: &[GraphSymbol]) -> String {
    symbols.iter().map(|s| s.as_char()).collect()
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Find the lane that currently expects `oid`, or allocate a new one.
fn find_or_allocate_lane(lanes: &mut Vec<Option<Oid>>, oid: Oid) -> usize {
    if let Some(idx) = lanes.iter().position(|l| *l == Some(oid)) {
        return idx;
    }
    allocate_lane(lanes, oid)
}

/// Place `oid` in the first free lane, or append a new lane.
fn allocate_lane(lanes: &mut Vec<Option<Oid>>, oid: Oid) -> usize {
    if let Some(idx) = lanes.iter().position(|l| l.is_none()) {
        lanes[idx] = Some(oid);
        idx
    } else {
        lanes.push(Some(oid));
        lanes.len() - 1
    }
}

/// Build a map from commit OID to the list of ref names pointing at it.
fn build_ref_map(repo: &Repository) -> Result<HashMap<Oid, Vec<String>>> {
    let mut map: HashMap<Oid, Vec<String>> = HashMap::new();

    if let Ok(references) = repo.references() {
        for reference in references.flatten() {
            if let Some(target) = reference.target() {
                if let Some(name) = reference.shorthand() {
                    // Skip HEAD itself (we handle it separately below).
                    if name != "HEAD" {
                        map.entry(target).or_default().push(name.to_string());
                    }
                }
            }
        }
    }

    // Also note HEAD if detached.
    if let Ok(head) = repo.head() {
        if repo.head_detached().unwrap_or(false) {
            if let Some(oid) = head.target() {
                map.entry(oid).or_default().push("HEAD".to_string());
            }
        }
    }

    Ok(map)
}

/// Format a duration as a human-readable relative time string.
fn format_relative_time(duration: chrono::Duration) -> String {
    let secs = duration.num_seconds();
    if secs < 60 {
        "just now".to_string()
    } else if secs < 3600 {
        let mins = secs / 60;
        if mins == 1 {
            "1 minute ago".to_string()
        } else {
            format!("{} minutes ago", mins)
        }
    } else if secs < 86400 {
        let hours = secs / 3600;
        if hours == 1 {
            "1 hour ago".to_string()
        } else {
            format!("{} hours ago", hours)
        }
    } else if secs < 2_592_000 {
        let days = secs / 86400;
        if days == 1 {
            "1 day ago".to_string()
        } else {
            format!("{} days ago", days)
        }
    } else if secs < 31_536_000 {
        let months = secs / 2_592_000;
        if months == 1 {
            "1 month ago".to_string()
        } else {
            format!("{} months ago", months)
        }
    } else {
        let years = secs / 31_536_000;
        if years == 1 {
            "1 year ago".to_string()
        } else {
            format!("{} years ago", years)
        }
    }
}
