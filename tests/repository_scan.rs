use agentctx::{project::Project, repository};
use std::process::Command;

#[test]
fn scans_deterministic_facts_from_tracked_files() {
    let directory = tempfile::tempdir().expect("temp directory");
    let root = directory.path();
    assert!(
        Command::new("git")
            .args(["init", "--quiet"])
            .current_dir(root)
            .status()
            .expect("git")
            .success()
    );
    std::fs::write(
        root.join("package.json"),
        r#"{"packageManager":"pnpm@10.0.0","scripts":{"test":"vitest"},"devDependencies":{"vitest":"1"}}"#,
    )
    .expect("package");
    std::fs::write(root.join("AGENTS.md"), "# Instructions").expect("instructions");
    std::fs::write(root.join(".gitignore"), "dist/\n.env\n").expect("gitignore");
    assert!(
        Command::new("git")
            .args(["add", "package.json", "AGENTS.md", ".gitignore"])
            .current_dir(root)
            .status()
            .expect("git add")
            .success()
    );

    let project = Project::detect(root).expect("project");
    let facts = repository::scan(&project).expect("scan");

    assert_eq!(facts.package_manager.as_deref(), Some("pnpm"));
    assert_eq!(facts.test_tools, ["vitest"]);
    assert_eq!(facts.commands, ["pnpm test"]);
    assert_eq!(facts.instructions, [std::path::PathBuf::from("AGENTS.md")]);
    assert_eq!(facts.generated_paths, ["dist/", ".env"]);
}
