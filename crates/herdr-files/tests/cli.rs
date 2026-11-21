//! Runs the built binary the way the mod does and checks stdout, stderr and
//! the exit code.

use std::process::{Command, Output};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_herdr-files"))
        .args(args)
        .output()
        .expect("the binary runs")
}

#[test]
fn files_prints_json() {
    let out = run(&["files", "main", "--cwd", env!("CARGO_MANIFEST_DIR")]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "stderr: {stderr}");
    assert!(stderr.is_empty(), "stderr: {stderr}");
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).expect("stdout is JSON");
    assert_eq!(json["results"], serde_json::json!([]));
    assert_eq!(json["truncated"], false);
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
