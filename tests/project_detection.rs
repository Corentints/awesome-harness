use agentctx::project::{Project, ProjectError};

#[test]
fn detects_the_nearest_git_repository_from_a_child() {
    let directory = tempfile::tempdir().expect("temp directory");
    std::fs::create_dir(directory.path().join(".git")).expect("git marker");
    let child = directory.path().join("src/nested");
    std::fs::create_dir_all(&child).expect("child directories");

    let project = Project::detect(&child).expect("project should be found");

    assert_eq!(project.root, directory.path().canonicalize().expect("root"));
    assert!(project.contains_path(&child.canonicalize().expect("child")));
}

#[test]
fn rejects_a_directory_outside_a_git_repository() {
    let directory = tempfile::tempdir().expect("temp directory");

    let error = Project::detect(directory.path()).expect_err("must not detect a project");

    assert!(matches!(error, ProjectError::NotGitRepository { .. }));
}
