use git2::{Oid, Repository, Signature};
use std::path::Path;
use tempfile::TempDir;

/// Create a fresh temp directory with an initialized git repo.
/// Sets user.name and user.email in the repo config.
pub fn create_test_repo() -> (TempDir, Repository) {
    let dir = TempDir::new().expect("failed to create temp dir");
    let repo = Repository::init(dir.path()).expect("failed to init repo");
    // Set config so commits work
    let mut config = repo.config().expect("failed to get config");
    config.set_str("user.name", "Test User").expect("set name");
    config
        .set_str("user.email", "test@example.com")
        .expect("set email");
    (dir, repo)
}

/// Create/overwrite a file, stage it, and commit.
pub fn commit_file(repo: &Repository, path: &str, content: &str, message: &str) -> Oid {
    let workdir = repo.workdir().expect("not bare");
    let full_path = workdir.join(path);
    if let Some(parent) = full_path.parent() {
        std::fs::create_dir_all(parent).expect("create dirs");
    }
    std::fs::write(&full_path, content).expect("write file");

    let mut index = repo.index().expect("open index");
    index.add_path(Path::new(path)).expect("add to index");
    index.write().expect("write index");
    let tree_oid = index.write_tree().expect("write tree");
    let tree = repo.find_tree(tree_oid).expect("find tree");
    let sig = Signature::now("Test User", "test@example.com").expect("signature");

    let parents: Vec<git2::Commit> = repo
        .head()
        .ok()
        .and_then(|h| h.peel_to_commit().ok())
        .into_iter()
        .collect();
    let parent_refs: Vec<&git2::Commit> = parents.iter().collect();

    repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parent_refs)
        .expect("commit")
}

/// Modify a file in the working directory without staging.
pub fn modify_file(repo: &Repository, path: &str, content: &str) {
    let workdir = repo.workdir().expect("not bare");
    let full_path = workdir.join(path);
    std::fs::write(&full_path, content).expect("write file");
}
