use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_herdr-files"))
        .args(args)
        .output()
        .expect("the binary runs")
}

struct TempDir(PathBuf);

impl TempDir {
    fn with(paths: &[&str]) -> TempDir {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!("herdr-files-{}-{n}", std::process::id()));
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

    fn write(&self, path: &str, text: &str) {
        fs::write(self.0.join(path), text).expect("write the file");
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn files(dir: &TempDir, args: &[&str]) -> serde_json::Value {
    succeed("files", dir, args)
}

fn grep(dir: &TempDir, args: &[&str]) -> serde_json::Value {
    succeed("grep", dir, args)
}

fn succeed(command: &str, dir: &TempDir, args: &[&str]) -> serde_json::Value {
    let mut all = vec![command, "--cwd", dir.path()];
    all.extend(args);
    let out = run(&all);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "stderr: {stderr}");
    assert!(stderr.is_empty(), "stderr: {stderr}");
    serde_json::from_slice(&out.stdout).expect("stdout is JSON")
}

fn paths(json: &serde_json::Value) -> Vec<String> {
    json["results"]
        .as_array()
        .expect("results is a list")
        .iter()
        .map(|r| r["path"].as_str().expect("path is a string").to_owned())
        .collect()
}

fn walked(dir: &TempDir, args: &[&str]) -> Vec<String> {
    let mut all = vec![""];
    all.extend(args);
    let mut found = paths(&files(dir, &all));
    found.sort();
    found
}

const REPO: &[&str] = &[
    ".git/HEAD",
    ".gitignore",
    ".github/ci.yml",
    "src/main.rs",
    "target/debug/app",
];

#[test]
fn files_lists_ignored_and_hidden_files() {
    let dir = TempDir::with(REPO);
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
    let dir = TempDir::with(REPO);
    assert_eq!(
        walked(&dir, &["--exclude", "target/", "--exclude", "*.yml"]),
        [".gitignore", "src/main.rs"]
    );
}

#[test]
fn files_prints_matches_best_first() {
    let dir = TempDir::with(&["docs/domain.md", "src/main.rs", "README.md"]);
    let json = files(&dir, &["main"]);
    assert_eq!(paths(&json), ["src/main.rs", "docs/domain.md"]);
    let scores: Vec<u64> = (json["results"].as_array().unwrap().iter())
        .map(|r| r["score"].as_u64().expect("score is a number"))
        .collect();
    assert!(scores[0] > scores[1], "scores: {scores:?}");
    assert_eq!(json["truncated"], false);
}

#[test]
fn files_prints_the_line_and_column_after_the_path() {
    let dir = TempDir::with(&["src/main.rs", "README.md"]);
    let json = files(&dir, &["main.rs:42:5"]);
    assert_eq!(paths(&json), ["src/main.rs"]);
    assert_eq!(json["line"], 42);
    assert_eq!(json["column"], 5);
    let json = files(&dir, &["main.rs"]);
    assert!(json["line"].is_null());
    assert!(json["column"].is_null());
}

#[test]
fn files_stops_at_the_limit() {
    let dir = TempDir::with(REPO);
    let json = files(&dir, &["", "--limit", "2"]);
    assert_eq!(paths(&json).len(), 2);
    assert_eq!(json["truncated"], true);
}

#[test]
fn files_rejects_a_bad_exclude() {
    let dir = TempDir::with(&[]);
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
fn grep_prints_matching_lines_in_path_order() {
    let dir = TempDir::with(&["src/main.rs", "README.md", "target/app.rs"]);
    dir.write("src/main.rs", "fn main() {\n    run();\n}\n");
    dir.write("README.md", "Run it with cargo run.\n");
    dir.write("target/app.rs", "run\n");
    let json = grep(&dir, &["run(", "--exclude", "target/"]);
    assert_eq!(
        json,
        serde_json::json!({
            "results": [{ "path": "src/main.rs", "line": 2, "column": 5, "text": "    run();" }],
            "truncated": false,
        })
    );
    let json = grep(&dir, &["run", "--exclude", "target/"]);
    assert_eq!(paths(&json), ["README.md", "src/main.rs"]);
}

#[test]
fn grep_takes_a_regex_and_case_options() {
    let dir = TempDir::with(&["a.txt"]);
    dir.write("a.txt", "Run\nrun\nr.n\n");
    let lines = |args: &[&str]| -> Vec<u64> {
        let json = grep(&dir, args);
        (json["results"].as_array().unwrap().iter())
            .map(|r| r["line"].as_u64().expect("line is a number"))
            .collect()
    };
    assert_eq!(lines(&["r.n"]), [3]);
    assert_eq!(lines(&["r.n", "--regex"]), [1, 2, 3]);
    assert_eq!(lines(&["r.n", "--regex", "--case-sensitive"]), [2, 3]);
}

#[test]
fn grep_stops_at_the_limit() {
    let dir = TempDir::with(&["a.txt", "b.txt"]);
    dir.write("a.txt", "x\nx\n");
    dir.write("b.txt", "x\n");
    let json = grep(&dir, &["x", "--limit", "2"]);
    assert_eq!(paths(&json), ["a.txt", "a.txt"]);
    assert_eq!(json["truncated"], true);
    let json = grep(&dir, &["x", "--limit", "3"]);
    assert_eq!(paths(&json).len(), 3);
    assert_eq!(json["truncated"], false);
}

#[test]
fn grep_skips_binary_files() {
    let dir = TempDir::with(&["app.bin", "main.rs"]);
    dir.write("app.bin", "main\0");
    dir.write("main.rs", "main\n");
    assert_eq!(paths(&grep(&dir, &["main"])), ["main.rs"]);
}

#[test]
fn grep_rejects_a_bad_regex() {
    let dir = TempDir::with(&[]);
    let out = run(&["grep", "a(", "--regex", "--cwd", dir.path()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stdout.is_empty());
    assert!(stderr.contains("bad pattern"), "stderr: {stderr}");
}

#[test]
fn usage_errors_exit_with_2() {
    for args in [&[][..], &["files"], &["grep"], &["no-such-command"]] {
        let out = run(args);
        assert_eq!(out.status.code(), Some(2), "args: {args:?}");
        assert!(out.stdout.is_empty(), "args: {args:?}");
    }
}
