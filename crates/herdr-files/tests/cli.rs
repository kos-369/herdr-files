//! Runs the built binary the way the mod does and checks stdout, stderr and
//! the exit code.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_herdr-files"))
        .args(args)
        .output()
        .expect("the binary runs")
}

/// A folder under the system temp folder, removed when the test ends.
struct TempDir(PathBuf);

impl TempDir {
    /// Creates the folder with `paths` in it as empty files. `name` must be
    /// unique per test, because tests run at the same time.
    fn with(name: &str, paths: &[&str]) -> TempDir {
        let root = std::env::temp_dir().join(format!("herdr-files-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create the folder");
        for path in paths {
            let path = root.join(path);
            fs::create_dir_all(path.parent().unwrap()).expect("create the parent");
            fs::write(&path, "").expect("create the file");
        }
        TempDir(root)
    }

    fn path(&self) -> &str {
        self.0.to_str().expect("the temp folder is UTF-8")
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// Runs `files` in `dir` with `args` (the query first) and returns the JSON
/// it printed.
fn files(dir: &TempDir, args: &[&str]) -> serde_json::Value {
    let mut all = vec!["files", "--cwd", dir.path()];
    all.extend(args);
    let out = run(&all);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "stderr: {stderr}");
    assert!(stderr.is_empty(), "stderr: {stderr}");
    serde_json::from_slice(&out.stdout).expect("stdout is JSON")
}

/// The paths in the output of `files`, in order.
fn paths(json: &serde_json::Value) -> Vec<String> {
    json["results"]
        .as_array()
        .expect("results is a list")
        .iter()
        .map(|r| r["path"].as_str().expect("path is a string").to_owned())
        .collect()
}

/// Every file `files` finds in `dir`, sorted by name. An empty query matches
/// every path.
fn walked(dir: &TempDir, args: &[&str]) -> Vec<String> {
    let mut all = vec![""];
    all.extend(args);
    let mut found = paths(&files(dir, &all));
    found.sort();
    found
}

// A small repository: .git, a .gitignore that ignores target/, a hidden
// folder, and an ignored build output.
const REPO: &[&str] = &[
    ".git/HEAD",
    ".gitignore",
    ".github/ci.yml",
    "src/main.rs",
    "target/debug/app",
];

#[test]
fn files_lists_ignored_and_hidden_files() {
    let dir = TempDir::with("ignored-and-hidden", REPO);
    fs::write(Path::new(dir.path()).join(".gitignore"), "target/\n").unwrap();
    assert_eq!(
        walked(&dir, &[]),
        [
            ".github/ci.yml",
            ".gitignore",
            "src/main.rs",
            "target/debug/app"
        ]
    );
}

#[test]
fn files_leaves_out_excludes() {
    let dir = TempDir::with("excludes", REPO);
    assert_eq!(
        walked(&dir, &["--exclude", "target/", "--exclude", "*.yml"]),
        [".gitignore", "src/main.rs"]
    );
}

#[test]
fn files_prints_matches_best_first() {
    let dir = TempDir::with(
        "best-first",
        &["docs/domain.md", "src/main.rs", "README.md"],
    );
    let json = files(&dir, &["main"]);
    assert_eq!(paths(&json), ["src/main.rs", "docs/domain.md"]);
    let scores: Vec<u64> = (json["results"].as_array().unwrap().iter())
        .map(|r| r["score"].as_u64().expect("score is a number"))
        .collect();
    assert!(scores[0] > scores[1], "scores: {scores:?}");
    assert_eq!(json["truncated"], false);
}

#[test]
fn files_stops_at_the_limit() {
    let dir = TempDir::with("limit", REPO);
    let json = files(&dir, &["", "--limit", "2"]);
    assert_eq!(paths(&json).len(), 2);
    assert_eq!(json["truncated"], true);
}

#[test]
fn files_rejects_a_bad_exclude() {
    let dir = TempDir::with("bad-exclude", &[]);
    let out = run(&["files", "main", "--cwd", dir.path(), "--exclude", "a["]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stdout.is_empty());
    assert!(stderr.contains("--exclude a["), "stderr: {stderr}");
}

#[test]
fn files_rejects_a_missing_folder() {
    let missing = concat!(env!("CARGO_MANIFEST_DIR"), "/no-such-folder");
    let out = run(&["files", "main", "--cwd", missing]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stdout.is_empty());
    assert!(stderr.contains("not a folder"), "stderr: {stderr}");
}

#[test]
fn usage_errors_exit_with_2() {
    for args in [&[][..], &["files"], &["no-such-command"]] {
        let out = run(args);
        assert_eq!(out.status.code(), Some(2), "args: {args:?}");
        assert!(out.stdout.is_empty(), "args: {args:?}");
    }
}
