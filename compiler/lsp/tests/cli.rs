use std::{
    io::Write,
    path::Path,
    process::{Command, Output, Stdio},
};

use serde_json::Value;

const SOURCE: &str = "package test.editor\r\nclass Box {\r\n    fun value(): Int = 42\r\n}\r\nfun identity(number: Int): Int = number\r\nfun use(`😀名`: Box): Int = identity(`😀名`.value())\r\n";

fn write(root: &Path, name: &str, text: &str) {
    let path = root.join(name);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

fn setup() -> tempfile::TempDir {
    let directory = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let root = directory.path();
    write(
        root,
        "core/Cone.toml",
        "schema = 1\n[cone]\ngroup = 'scoop'\nname = 'scoop.core'\nversion = '0.1.0'\nkind = 'library'\n",
    );
    write(
        root,
        "core/src/core.scoop",
        "package scoop.core\npublic class Marker\n",
    );
    write(
        root,
        "app/Cone.toml",
        "schema = 1\n[cone]\ngroup = 'test'\nname = 'editor'\nversion = '1.0.0'\nkind = 'library'\n[dependencies]\n'scoop:scoop.core' = {version = '0.1.0', path = '../core'}\n",
    );
    write(root, "app/src/value.scoop", SOURCE);
    directory
}

fn run(root: &Path, args: &[&str], buffer: Option<&str>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_scoop-lsp"));
    command
        .current_dir(root.join("app"))
        .args(["query", "--package-cache-dir"])
        .arg(root.join("cache"))
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(buffer) = buffer {
        let mut child = command.stdin(Stdio::piped()).spawn().unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(buffer.as_bytes())
            .unwrap();
        child.wait_with_output().unwrap()
    } else {
        command.stdin(Stdio::null()).output().unwrap()
    }
}

fn query(root: &Path, args: &[&str], buffer: Option<&str>) -> Value {
    let mut args = args.to_vec();
    args.extend(["--format", "json"]);
    let output = run(root, &args, buffer);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["schema"], 1);
    assert_eq!(value["kind"], "query");
    assert_eq!(value["rangeBase"], 0);
    value
}

#[test]
fn buffer_and_new_sessions_observe_edits_without_writing_source() {
    let directory = setup();
    let root = directory.path();
    let overlay = "package test.editor\nfun overlay(): Int = 7\n";
    let symbols = query(
        root,
        &["symbols", "src/value.scoop", "--stdin"],
        Some(overlay),
    );
    assert_eq!(symbols["result"][0]["name"], "overlay");
    assert_eq!(
        std::fs::read_to_string(root.join("app/src/value.scoop")).unwrap(),
        SOURCE
    );
    write(
        root,
        "app/src/value.scoop",
        "package test.editor\nfun diskEdit(): Int = 8\n",
    );
    let symbols = query(root, &["symbols", "src/value.scoop"], None);
    assert_eq!(symbols["result"][0]["name"], "diskEdit");
    let symbols = query(
        root,
        &["symbols", "src/new.scoop", "--stdin"],
        Some(overlay),
    );
    assert_eq!(symbols["result"][0]["name"], "overlay");
    assert!(!root.join("app/src/new.scoop").exists());
}

#[test]
fn source_and_project_errors_keep_partial_information_and_diagnostics() {
    let directory = setup();
    let root = directory.path();
    let broken = "package test.editor\nfun valid(): Int = 1\nfun broken(value: ) {}\n";
    let symbols = query(
        root,
        &["symbols", "src/value.scoop", "--stdin"],
        Some(broken),
    );
    assert_eq!(symbols["status"], "partial", "{symbols:#}");
    assert_eq!(symbols["result"][0]["name"], "valid");
    assert!(
        symbols["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["version"] == 1 && !d["diagnostics"].as_array().unwrap().is_empty())
    );
    write(root, "app/Cone.toml", "schema = broken\n");
    let symbols = query(root, &["symbols", "src/value.scoop"], None);
    assert_eq!(symbols["status"], "partial");
    assert!(symbols["diagnostics"].to_string().contains("SCOOP_PROJECT"));
    assert_eq!(symbols["result"][0]["name"], "Box");
    let manifest = query(root, &["diagnostics", "Cone.toml"], None);
    assert_eq!(manifest["status"], "partial");
    assert!(manifest["result"].to_string().contains("SCOOP_PROJECT"));
}

#[test]
fn purpose_target_and_input_failures_are_explicit() {
    let directory = setup();
    let root = directory.path();
    write(
        root,
        "app/tests/local.scoop",
        "package test.editor\nfun testOnly(): Int = 9\n",
    );
    let production = query(root, &["workspace-symbols", "testOnly"], None);
    assert_eq!(production["status"], "empty");
    let tests = query(
        root,
        &["workspace-symbols", "testOnly", "--purpose", "test"],
        None,
    );
    assert_eq!(tests["result"][0]["name"], "testOnly");
    let unsupported = query(
        root,
        &["symbols", "src/value.scoop", "--target", "unknown-cpu"],
        None,
    );
    assert_eq!(unsupported["status"], "partial");
    assert!(
        unsupported["diagnostics"]
            .to_string()
            .contains("unsupported target")
    );
    for (args, code) in [
        (vec!["symbols", "missing.scoop", "--format", "json"], 1),
        (
            vec![
                "hover",
                "src/value.scoop",
                "--line",
                "0",
                "--column",
                "1",
                "--format=json",
            ],
            2,
        ),
        (
            vec![
                "hover",
                "src/value.scoop",
                "--line",
                "999",
                "--column",
                "1",
                "--format=json",
            ],
            1,
        ),
    ] {
        let output = run(root, &args, None);
        assert_eq!(output.status.code(), Some(code));
        assert!(output.stdout.is_empty());
        let error: Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(error["kind"], "error");
    }
    let output = run(
        root,
        &["capabilities", "--project", "missing", "--format=json"],
        None,
    );
    assert!(output.status.success());
    let result: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["result"]["capabilities"]["hoverProvider"], true);
    assert!(!root.join("target").exists());
}
