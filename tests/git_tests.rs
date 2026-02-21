mod helpers;

use gittui::git::{branch, commit, diff, log, stash, status};

// ============================================================================
// Status tests
// ============================================================================

#[test]
fn test_status_empty_repo() {
    let (_dir, repo) = helpers::create_test_repo();
    let (staged, unstaged, untracked, conflicts) = status::get_status(&repo).unwrap();
    assert!(staged.is_empty());
    assert!(unstaged.is_empty());
    assert!(untracked.is_empty());
    assert!(conflicts.is_empty());
}

#[test]
fn test_status_untracked_file() {
    let (dir, repo) = helpers::create_test_repo();
    // Create a file but don't stage it
    std::fs::write(dir.path().join("new.txt"), "hello").unwrap();
    let (staged, unstaged, untracked, _) = status::get_status(&repo).unwrap();
    assert!(staged.is_empty());
    assert!(unstaged.is_empty());
    assert_eq!(untracked.len(), 1);
    assert_eq!(untracked[0].path, "new.txt");
}

#[test]
fn test_status_stage_file() {
    let (_dir, repo) = helpers::create_test_repo();
    helpers::commit_file(&repo, "a.txt", "initial", "init");
    helpers::modify_file(&repo, "a.txt", "modified");

    // Before staging
    let (staged, unstaged, _, _) = status::get_status(&repo).unwrap();
    assert!(staged.is_empty());
    assert_eq!(unstaged.len(), 1);

    // Stage the file
    status::stage_file(&repo, "a.txt").unwrap();
    let (staged, unstaged, _, _) = status::get_status(&repo).unwrap();
    assert_eq!(staged.len(), 1);
    assert!(unstaged.is_empty());
}

#[test]
fn test_status_unstage_file() {
    let (_dir, repo) = helpers::create_test_repo();
    helpers::commit_file(&repo, "a.txt", "initial", "init");
    helpers::modify_file(&repo, "a.txt", "modified");
    status::stage_file(&repo, "a.txt").unwrap();

    // Verify staged
    let (staged, _, _, _) = status::get_status(&repo).unwrap();
    assert_eq!(staged.len(), 1);

    // Unstage
    status::unstage_file(&repo, "a.txt").unwrap();
    let (staged, unstaged, _, _) = status::get_status(&repo).unwrap();
    assert!(staged.is_empty());
    assert_eq!(unstaged.len(), 1);
}

#[test]
fn test_status_stage_all() {
    let (dir, repo) = helpers::create_test_repo();
    helpers::commit_file(&repo, "a.txt", "a", "init a");
    helpers::modify_file(&repo, "a.txt", "modified a");
    std::fs::write(dir.path().join("b.txt"), "new b").unwrap();

    status::stage_all(&repo).unwrap();
    let (staged, unstaged, untracked, _) = status::get_status(&repo).unwrap();
    assert_eq!(staged.len(), 2); // modified a.txt + new b.txt
    assert!(unstaged.is_empty());
    assert!(untracked.is_empty());
}

#[test]
fn test_status_unstage_all() {
    let (dir, repo) = helpers::create_test_repo();
    helpers::commit_file(&repo, "a.txt", "a", "init a");
    helpers::modify_file(&repo, "a.txt", "modified a");
    std::fs::write(dir.path().join("b.txt"), "new b").unwrap();

    status::stage_all(&repo).unwrap();
    status::unstage_all(&repo).unwrap();
    let (staged, _, _, _) = status::get_status(&repo).unwrap();
    assert!(staged.is_empty());
}

#[test]
fn test_status_deleted_file() {
    let (_dir, repo) = helpers::create_test_repo();
    helpers::commit_file(&repo, "a.txt", "content", "add a");
    let workdir = repo.workdir().unwrap();
    std::fs::remove_file(workdir.join("a.txt")).unwrap();

    let (_, unstaged, _, _) = status::get_status(&repo).unwrap();
    assert_eq!(unstaged.len(), 1);
    assert_eq!(unstaged[0].status.to_string(), "deleted");
}

#[test]
fn test_status_discard_changes() {
    let (_dir, repo) = helpers::create_test_repo();
    helpers::commit_file(&repo, "a.txt", "original", "init");
    helpers::modify_file(&repo, "a.txt", "modified");

    status::discard_changes(&repo, "a.txt").unwrap();
    let content = std::fs::read_to_string(repo.workdir().unwrap().join("a.txt")).unwrap();
    assert_eq!(content, "original");
}

// ============================================================================
// Commit tests
// ============================================================================

#[test]
fn test_commit_initial() {
    let (_dir, repo) = helpers::create_test_repo();
    let oid = helpers::commit_file(&repo, "a.txt", "hello", "initial commit");
    let c = repo.find_commit(oid).unwrap();
    assert_eq!(c.summary().unwrap(), "initial commit");
}

#[test]
fn test_commit_with_staged_files() {
    let (_dir, repo) = helpers::create_test_repo();
    helpers::commit_file(&repo, "a.txt", "a", "first");
    helpers::modify_file(&repo, "a.txt", "modified");
    status::stage_file(&repo, "a.txt").unwrap();

    commit::commit(&repo, "second commit").unwrap();
    let head = repo.head().unwrap().peel_to_commit().unwrap();
    assert_eq!(head.summary().unwrap(), "second commit");
}

#[test]
fn test_commit_amend() {
    let (_dir, repo) = helpers::create_test_repo();
    helpers::commit_file(&repo, "a.txt", "a", "original message");

    commit::amend(&repo, "amended message").unwrap();
    let head = repo.head().unwrap().peel_to_commit().unwrap();
    assert_eq!(head.summary().unwrap(), "amended message");
}

#[test]
fn test_commit_empty_message_rejected() {
    let (_dir, repo) = helpers::create_test_repo();
    helpers::commit_file(&repo, "a.txt", "a", "first");
    helpers::modify_file(&repo, "a.txt", "modified");
    status::stage_file(&repo, "a.txt").unwrap();

    let result = commit::commit(&repo, "   ");
    assert!(result.is_err());
}

// ============================================================================
// Branch tests
// ============================================================================

#[test]
fn test_branch_list() {
    let (_dir, repo) = helpers::create_test_repo();
    helpers::commit_file(&repo, "a.txt", "a", "init");

    let branches = branch::list_branches(&repo).unwrap();
    // Should have at least the default branch
    assert!(!branches.is_empty());
    assert!(branches.iter().any(|b| b.is_current));
}

#[test]
fn test_branch_create() {
    let (_dir, repo) = helpers::create_test_repo();
    helpers::commit_file(&repo, "a.txt", "a", "init");

    branch::create_branch(&repo, "feature").unwrap();
    let branches = branch::list_branches(&repo).unwrap();
    assert!(branches.iter().any(|b| b.name == "feature"));
}

#[test]
fn test_branch_delete() {
    let (_dir, repo) = helpers::create_test_repo();
    helpers::commit_file(&repo, "a.txt", "a", "init");
    branch::create_branch(&repo, "to-delete").unwrap();

    branch::delete_branch(&repo, "to-delete").unwrap();
    let branches = branch::list_branches(&repo).unwrap();
    assert!(!branches.iter().any(|b| b.name == "to-delete"));
}

#[test]
fn test_branch_checkout() {
    let (_dir, repo) = helpers::create_test_repo();
    helpers::commit_file(&repo, "a.txt", "a", "init");
    branch::create_branch(&repo, "other").unwrap();

    branch::checkout_branch(&repo, "other").unwrap();
    let head = repo.head().unwrap();
    assert_eq!(head.shorthand().unwrap(), "other");
}

#[test]
fn test_branch_rename() {
    let (_dir, repo) = helpers::create_test_repo();
    helpers::commit_file(&repo, "a.txt", "a", "init");
    branch::create_branch(&repo, "old-name").unwrap();

    branch::rename_branch(&repo, "old-name", "new-name").unwrap();
    let branches = branch::list_branches(&repo).unwrap();
    assert!(!branches.iter().any(|b| b.name == "old-name"));
    assert!(branches.iter().any(|b| b.name == "new-name"));
}

#[test]
fn test_branch_delete_current_fails() {
    let (_dir, repo) = helpers::create_test_repo();
    helpers::commit_file(&repo, "a.txt", "a", "init");

    // The default branch is current; deleting it should fail
    let head = repo.head().unwrap();
    let current_name = head.shorthand().unwrap().to_string();
    let result = branch::delete_branch(&repo, &current_name);
    assert!(result.is_err());
}

// ============================================================================
// Diff tests
// ============================================================================

#[test]
fn test_diff_unstaged() {
    let (_dir, repo) = helpers::create_test_repo();
    helpers::commit_file(&repo, "a.txt", "line1\nline2\n", "init");
    helpers::modify_file(&repo, "a.txt", "line1\nmodified\n");

    let d = diff::get_unstaged_diff(&repo, "a.txt").unwrap();
    assert!(!d.hunks.is_empty());
    assert!(d.additions > 0 || d.deletions > 0);
}

#[test]
fn test_diff_staged() {
    let (_dir, repo) = helpers::create_test_repo();
    helpers::commit_file(&repo, "a.txt", "original\n", "init");
    helpers::modify_file(&repo, "a.txt", "changed\n");
    status::stage_file(&repo, "a.txt").unwrap();

    let d = diff::get_staged_diff(&repo, "a.txt").unwrap();
    assert!(!d.hunks.is_empty());
}

#[test]
fn test_diff_additions_deletions_count() {
    let (_dir, repo) = helpers::create_test_repo();
    helpers::commit_file(&repo, "a.txt", "line1\nline2\nline3\n", "init");
    helpers::modify_file(&repo, "a.txt", "line1\nnewline\nline3\nextra\n");

    let d = diff::get_unstaged_diff(&repo, "a.txt").unwrap();
    // line2 deleted, newline added, extra added => 2 additions, 1 deletion
    assert!(d.additions >= 1);
    assert!(d.deletions >= 1);
}

#[test]
fn test_diff_no_changes_empty() {
    let (_dir, repo) = helpers::create_test_repo();
    helpers::commit_file(&repo, "a.txt", "unchanged\n", "init");

    // No modifications -- diff should have no hunks
    let d = diff::get_unstaged_diff(&repo, "a.txt").unwrap();
    assert!(d.hunks.is_empty());
    assert_eq!(d.additions, 0);
    assert_eq!(d.deletions, 0);
}

// ============================================================================
// Log tests
// ============================================================================

#[test]
fn test_log_empty_repo() {
    let (_dir, repo) = helpers::create_test_repo();
    // No commits yet
    let log_entries = log::get_log(&repo, 100).unwrap();
    assert!(log_entries.is_empty());
}

#[test]
fn test_log_single_commit() {
    let (_dir, repo) = helpers::create_test_repo();
    helpers::commit_file(&repo, "a.txt", "a", "first commit");

    let log_entries = log::get_log(&repo, 100).unwrap();
    assert_eq!(log_entries.len(), 1);
    assert_eq!(log_entries[0].summary, "first commit");
}

#[test]
fn test_log_multiple_commits() {
    let (_dir, repo) = helpers::create_test_repo();
    helpers::commit_file(&repo, "a.txt", "a", "first");
    helpers::commit_file(&repo, "b.txt", "b", "second");
    helpers::commit_file(&repo, "c.txt", "c", "third");

    let log_entries = log::get_log(&repo, 100).unwrap();
    assert_eq!(log_entries.len(), 3);
    // Most recent first
    assert_eq!(log_entries[0].summary, "third");
    assert_eq!(log_entries[1].summary, "second");
    assert_eq!(log_entries[2].summary, "first");
}

#[test]
fn test_log_max_count() {
    let (_dir, repo) = helpers::create_test_repo();
    helpers::commit_file(&repo, "a.txt", "a", "first");
    helpers::commit_file(&repo, "b.txt", "b", "second");
    helpers::commit_file(&repo, "c.txt", "c", "third");

    let log_entries = log::get_log(&repo, 2).unwrap();
    assert_eq!(log_entries.len(), 2);
    assert_eq!(log_entries[0].summary, "third");
    assert_eq!(log_entries[1].summary, "second");
}

#[test]
fn test_log_ref_decorations() {
    let (_dir, repo) = helpers::create_test_repo();
    helpers::commit_file(&repo, "a.txt", "a", "init");
    branch::create_branch(&repo, "feature-x").unwrap();

    let log_entries = log::get_log(&repo, 100).unwrap();
    // HEAD commit should have refs
    let head_entry = &log_entries[0];
    assert!(head_entry.is_head || !head_entry.refs.is_empty());
}

#[test]
fn test_log_head_flag() {
    let (_dir, repo) = helpers::create_test_repo();
    helpers::commit_file(&repo, "a.txt", "a", "init");
    helpers::commit_file(&repo, "b.txt", "b", "second");

    let log_entries = log::get_log(&repo, 100).unwrap();
    // First entry in the walk should be HEAD
    assert!(log_entries[0].is_head);
    assert!(!log_entries[1].is_head);
}

#[test]
fn test_log_commit_info_fields() {
    let (_dir, repo) = helpers::create_test_repo();
    helpers::commit_file(&repo, "a.txt", "a", "test message");

    let log_entries = log::get_log(&repo, 100).unwrap();
    let entry = &log_entries[0];
    assert_eq!(entry.summary, "test message");
    assert_eq!(entry.author, "Test User");
    assert_eq!(entry.author_email, "test@example.com");
    assert!(!entry.oid.is_empty());
    assert!(!entry.short_oid.is_empty());
    assert_eq!(entry.short_oid.len(), 7);
    assert_eq!(entry.parent_count, 0); // first commit has no parents
}

// ============================================================================
// Stash tests
// ============================================================================

#[test]
fn test_stash_save_and_list() {
    let (_dir, repo) = helpers::create_test_repo();
    helpers::commit_file(&repo, "a.txt", "original", "init");
    helpers::modify_file(&repo, "a.txt", "modified for stash");

    let mut repo = repo; // Need mut for stash operations
    stash::stash_save(&mut repo, Some("test stash")).unwrap();

    let stashes = stash::list_stashes(&mut repo).unwrap();
    assert_eq!(stashes.len(), 1);
    assert!(stashes[0].message.contains("test stash"));

    // Working tree should be clean after stash
    let content = std::fs::read_to_string(repo.workdir().unwrap().join("a.txt")).unwrap();
    assert_eq!(content, "original");
}

#[test]
fn test_stash_pop() {
    let (_dir, repo) = helpers::create_test_repo();
    helpers::commit_file(&repo, "a.txt", "original", "init");
    helpers::modify_file(&repo, "a.txt", "stashed content");

    let mut repo = repo;
    stash::stash_save(&mut repo, Some("to pop")).unwrap();
    stash::stash_pop(&mut repo).unwrap();

    // Stash list should be empty
    let stashes = stash::list_stashes(&mut repo).unwrap();
    assert!(stashes.is_empty());

    // Changes should be restored
    let content = std::fs::read_to_string(repo.workdir().unwrap().join("a.txt")).unwrap();
    assert_eq!(content, "stashed content");
}

#[test]
fn test_stash_drop() {
    let (_dir, repo) = helpers::create_test_repo();
    helpers::commit_file(&repo, "a.txt", "original", "init");
    helpers::modify_file(&repo, "a.txt", "to drop");

    let mut repo = repo;
    stash::stash_save(&mut repo, Some("to drop")).unwrap();
    stash::stash_drop(&mut repo, 0).unwrap();

    let stashes = stash::list_stashes(&mut repo).unwrap();
    assert!(stashes.is_empty());
}

#[test]
fn test_stash_multiple() {
    let (_dir, repo) = helpers::create_test_repo();
    helpers::commit_file(&repo, "a.txt", "original", "init");

    let mut repo = repo;

    // First stash
    helpers::modify_file(&repo, "a.txt", "change 1");
    stash::stash_save(&mut repo, Some("stash one")).unwrap();

    // Second stash
    helpers::modify_file(&repo, "a.txt", "change 2");
    stash::stash_save(&mut repo, Some("stash two")).unwrap();

    let stashes = stash::list_stashes(&mut repo).unwrap();
    assert_eq!(stashes.len(), 2);
    // Most recent stash is at index 0
    assert!(stashes[0].message.contains("stash two"));
    assert!(stashes[1].message.contains("stash one"));
}

#[test]
fn test_stash_apply() {
    let (_dir, repo) = helpers::create_test_repo();
    helpers::commit_file(&repo, "a.txt", "original", "init");
    helpers::modify_file(&repo, "a.txt", "for apply");

    let mut repo = repo;
    stash::stash_save(&mut repo, Some("apply test")).unwrap();
    stash::stash_apply(&mut repo, 0).unwrap();

    // Stash should still be in the list (apply does not drop)
    let stashes = stash::list_stashes(&mut repo).unwrap();
    assert_eq!(stashes.len(), 1);

    // Changes should be restored
    let content = std::fs::read_to_string(repo.workdir().unwrap().join("a.txt")).unwrap();
    assert_eq!(content, "for apply");
}
