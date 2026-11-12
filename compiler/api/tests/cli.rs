use scoop_api::*;
use std::path::Path;
use std::process::{Command, Output};

fn write(path: &Path, value: &impl serde::Serialize) {
    std::fs::write(path, serde_json::to_vec(value).unwrap()).unwrap();
}

fn site() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("api")).unwrap();
    write(
        &root.path().join("api/index.json"),
        &Index {
            schema: SCHEMA,
            target: "aarch64-apple-darwin".into(),
            cones: vec![ConeReference {
                key: "library".into(),
                coordinate: "examples:library:1.0.0".into(),
                file: "api/library.json".into(),
            }],
        },
    );
    let declaration = |key: &str, signature: &str| Declaration {
        key: key.into(),
        alternate_keys: vec![],
        parent: None,
        package: vec!["examples".into()],
        name: "choose".into(),
        qualified_name: "examples.choose".into(),
        kind: "function".into(),
        visibility: "public".into(),
        property: None,
        signature: signature.into(),
        receiver: Some("Option<T>".into()),
        equality_requirements: None,
        aliases: vec!["惰性默认值".into()],
        source: Source {
            path: Some("src/选择.scoop".into()),
            line: 3,
            byte_start: 20,
            byte_end: 40,
            url: Some("source.html#L3".into()),
        },
        url: format!("choose.html#{key}"),
        documentation: Some(Documentation {
            markdown: "Evaluates fallback only when absent.".into(),
            tags: vec![],
        }),
        relations: vec![],
    };
    write(
        &root.path().join("api/library.json"),
        &Cone {
            schema: SCHEMA,
            coordinate: "examples:library:1.0.0".into(),
            declarations: vec![
                declaration(
                    "nominal",
                    "public fun <T: Equals<T>> Option<T>.choose(): T?",
                ),
                declaration(
                    "structural",
                    "public fun <T> Option<T>.choose(): T? where requires(value: T) { value.equals(value): Boolean }",
                ),
            ],
            bindings: vec![
                Binding {
                    package: vec!["public_api".into()],
                    name: "pick".into(),
                    target: "structural".into(),
                    reexport: true,
                },
                Binding {
                    package: vec!["external".into()],
                    name: "MissingSource".into(),
                    target: "external".into(),
                    reexport: true,
                },
            ],
        },
    );
    root
}

fn invoke(root: &Path, arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_scoop-api"))
        .arg("--docs")
        .arg(root)
        .args(arguments)
        .output()
        .unwrap()
}

#[test]
fn queries_preserve_overloads_imports_unicode_and_pagination() {
    let root = site();
    let output = invoke(root.path(), &["show", "choose", "--format", "json"]);
    assert!(output.status.success() && output.stderr.is_empty());
    let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["total"], 2);
    assert!(result["items"].as_array().unwrap().iter().any(|entry| {
        entry["declaration"]["signature"]
            .as_str()
            .unwrap()
            .contains("where requires")
    }));
    let alias = invoke(
        root.path(),
        &[
            "show",
            "public_api.pick",
            "--cone",
            "library",
            "--format=json",
        ],
    );
    let alias: serde_json::Value = serde_json::from_slice(&alias.stdout).unwrap();
    assert_eq!(alias["total"], 1);
    assert_eq!(alias["items"][0]["imports"][0]["reexport"], true);
    assert_eq!(
        alias["items"][0]["declaration"]["source"]["path"],
        "src/选择.scoop"
    );
    let page = invoke(
        root.path(),
        &[
            "search",
            "Option 惰性",
            "--limit",
            "1",
            "--offset",
            "1",
            "--format=json",
        ],
    );
    let page: serde_json::Value = serde_json::from_slice(&page.stdout).unwrap();
    assert_eq!(page["total"], 2);
    assert_eq!(page["items"].as_array().unwrap().len(), 1);
    let missing = invoke(root.path(), &["show", "MissingSource", "--format=json"]);
    let missing: serde_json::Value = serde_json::from_slice(&missing.stdout).unwrap();
    assert_eq!(missing["total"], 1);
    assert!(missing["items"][0]["declaration"].is_null());
}

#[test]
fn empty_results_and_errors_use_the_documented_streams() {
    let root = site();
    let empty = invoke(root.path(), &["search", "does not exist", "--format=json"]);
    assert!(empty.status.success() && empty.stderr.is_empty());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&empty.stdout).unwrap()["status"],
        "empty"
    );
    for (path, arguments, code) in [
        (
            root.path().to_owned(),
            vec![
                "show",
                "choose",
                "--target",
                "x86_64-unknown-linux-gnu",
                "--format=json",
            ],
            1,
        ),
        (
            root.path().join("not-published"),
            vec!["show", "choose", "--format=json"],
            1,
        ),
        (
            root.path().to_owned(),
            vec!["search", "", "--format=json"],
            2,
        ),
        (
            root.path().to_owned(),
            vec!["search", "choose", "--limit", "0", "--format=json"],
            2,
        ),
    ] {
        let result = invoke(&path, &arguments);
        assert_eq!(result.status.code(), Some(code));
        assert!(result.stdout.is_empty());
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&result.stderr).unwrap()["code"],
            code
        );
    }
}

#[test]
fn help_is_available_before_documentation_and_schema_errors_are_explicit() {
    let help = Command::new(env!("CARGO_BIN_EXE_scoop-api"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(help.status.success());
    let root = site();
    write(
        &root.path().join("api/index.json"),
        &Index {
            schema: 99,
            target: "aarch64-apple-darwin".into(),
            cones: vec![],
        },
    );
    let output = invoke(root.path(), &["show", "choose"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("unsupported API schema 99")
    );
}
