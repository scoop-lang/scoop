use std::{fs, process::Command};

#[test]
fn creates_and_lists_a_virtual_workspace_without_a_toolchain() {
    let directory = tempfile::tempdir().unwrap();
    let destination = directory.path().join("project");
    let executable = env!("CARGO_BIN_EXE_scoop-pm");
    let output = Command::new(executable)
        .args(["new", "--workspace"])
        .arg(&destination)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let output = Command::new(executable)
        .args(["members", "--json"])
        .current_dir(&destination)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["members"].as_array().unwrap().len(), 0);
    let before = fs::read(destination.join("Cone.toml")).unwrap();
    let output = Command::new(executable)
        .args(["new", "--workspace"])
        .arg(&destination)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert_eq!(fs::read(destination.join("Cone.toml")).unwrap(), before);
}

#[test]
fn malformed_manifest_reports_its_path_and_location() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(
        directory.path().join("Cone.toml"),
        "schema = 1\n[workspace]\nmembers = ['../outside']\n",
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_scoop-pm"))
        .arg("members")
        .current_dir(directory.path())
        .output()
        .unwrap();
    assert!(!output.status.success());
    let error = String::from_utf8(output.stderr).unwrap();
    assert!(error.contains("Cone.toml") && error.contains("bytes") && error.contains("escape"));
}

#[test]
fn members_delivers_inherited_metadata_and_bundle_without_loading_resources() {
    let directory = tempfile::tempdir().unwrap();
    fs::create_dir(directory.path().join("app")).unwrap();
    fs::write(
        directory.path().join("Cone.toml"),
        "schema = 1\n[workspace]\nmembers = ['app']\n[workspace.cone]\n\
         group = 'dev.example'\nversion = '1.2.3'\nlicense = 'MIT'\n",
    )
    .unwrap();
    fs::write(
        directory.path().join("app/Cone.toml"),
        "schema = 1\n[cone]\ngroup = { workspace = true }\nname = 'app'\n\
         version = { workspace = true }\nkind = 'executable'\n\
         license = { workspace = true }\ndescription = 'A bundle consumer'\n\
         [bundle]\nconfig = 'packaging/Bundle.toml'\n",
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_scoop-pm"))
        .args(["members", "--json"])
        .current_dir(directory.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let member = &result["members"][0];
    assert_eq!(member["coordinate"], "dev.example:app:1.2.3");
    assert_eq!(member["metadata"]["license"], "MIT");
    assert_eq!(member["metadata"]["description"], "A bundle consumer");
    assert_eq!(member["bundle"]["table"]["config"], "packaging/Bundle.toml");
    assert!(!directory.path().join("Cone.lock").exists());
}
