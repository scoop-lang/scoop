use clap::Parser;
use serde_json::json;
use std::{collections::BTreeMap, fs, path::Path};

use crate::{Args, cache, config, deploy::Plan, target::Target};

#[test]
fn existing_binary_requires_target_config_and_rejects_build_options() {
    assert!(Args::try_parse_from(["bundle", "--executable", "app"]).is_err());
    assert!(
        Args::try_parse_from([
            "bundle",
            "--executable",
            "app",
            "--config",
            "Bundle.toml",
            "--target",
            "aarch64-apple-darwin",
            "--profile",
            "release"
        ])
        .is_err()
    );
    assert!(
        Args::try_parse_from([
            "bundle",
            "--executable",
            "app",
            "--config",
            "Bundle.toml",
            "--target",
            "aarch64-apple-darwin"
        ])
        .is_ok()
    );
}

#[test]
fn external_metadata_inherits_only_missing_cone_values_and_resolves_paths_at_source() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("Bundle.toml");
    fs::write(&source,"[bundle]\nschema=1\nidentifier='dev.example.editor'\nname='editor-release'\n[[bundle.files]]\nfrom='../assets'\nto='assets'\nkind='resource'\nwhen={os=['darwin','linux'],env='none'}\n").unwrap();
    let loaded = config::Loaded::finish(
        config::parse_external(&source).unwrap(),
        source,
        Some("editor"),
        Some("1.2.3"),
        &BTreeMap::from([("license".into(), "MIT".into())]),
    )
    .unwrap();
    assert_eq!(loaded.name, "editor-release");
    assert_eq!(loaded.version.to_string(), "1.2.3");
    assert_eq!(loaded.config.license.as_deref(), Some("MIT"));
    assert_eq!(
        loaded.path(&loaded.config.files[0].from),
        root.path().join("../assets")
    );
    assert!(loaded.config.files[0].matches(&Target::parse("aarch64-apple-darwin").unwrap()));
    assert!(!loaded.config.files[0].matches(&Target::parse("x86_64-unknown-linux-gnu").unwrap()));
}

#[test]
fn configuration_rejects_typographical_errors_and_invalid_destinations() {
    let root = tempfile::tempdir().unwrap();
    let file = root.path().join("Bundle.toml");
    fs::write(
        &file,
        "[bundle]\nschema=1\nidentifier='dev.example.app'\ndisplayname='wrong'\n",
    )
    .unwrap();
    assert!(format!("{:#}", config::parse_external(&file).unwrap_err()).contains("displayname"));
    for path in [
        "",
        "../escape",
        "a/../../escape",
        "/absolute",
        "C:\\absolute",
        ".",
    ] {
        assert!(config::relative_destination(path).is_err(), "{path}");
    }
    assert_eq!(
        config::relative_destination("images/../assets/icon.png").unwrap(),
        Path::new("assets/icon.png")
    );
}

#[test]
fn plan_detects_generated_file_and_platform_case_conflicts() {
    let mut plan = Plan::new(&Target::parse("aarch64-apple-darwin").unwrap());
    plan.generated("Contents/Info.plist", b"first".to_vec())
        .unwrap();
    assert!(plan.generated("Contents/info.plist", Vec::new()).is_err());
    assert!(
        plan.generated("Contents/Info.plist/child", Vec::new())
            .is_err()
    );
    assert!(plan.generated("Contents/Info.plist", Vec::new()).is_err());
}

#[test]
fn input_changes_and_new_search_candidates_invalidate_a_completed_bundle() {
    let root = tempfile::tempdir().unwrap();
    let input = root.path().join("input");
    let candidate = root.path().join("new-library");
    let output = root.path().join("output");
    fs::write(&input, b"first").unwrap();
    fs::create_dir(&output).unwrap();
    fs::write(output.join("app"), b"complete").unwrap();
    let mut inputs = cache::Inputs::default();
    inputs.watch(&input).unwrap();
    inputs.watch(&candidate).unwrap();
    inputs
        .save(&output, json!({"format":"test"}), "app".into())
        .unwrap();
    assert!(
        cache::hit(&output, &json!({"format":"test"}))
            .unwrap()
            .is_some()
    );
    fs::write(&candidate, b"new").unwrap();
    assert!(
        cache::hit(&output, &json!({"format":"test"}))
            .unwrap()
            .is_none()
    );
    let mut inputs = cache::Inputs::default();
    inputs.watch(&input).unwrap();
    inputs
        .save(&output, json!({"format":"test"}), "app".into())
        .unwrap();
    fs::write(&input, b"changed content").unwrap();
    assert!(
        cache::hit(&output, &json!({"format":"test"}))
            .unwrap()
            .is_none()
    );
}

#[cfg(unix)]
#[test]
fn tree_deployment_preserves_framework_links_and_executable_permissions() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("input");
    fs::create_dir_all(source.join("Versions/A")).unwrap();
    fs::write(source.join("Versions/A/Helper"), b"program").unwrap();
    fs::set_permissions(
        source.join("Versions/A/Helper"),
        fs::Permissions::from_mode(0o751),
    )
    .unwrap();
    symlink("A", source.join("Versions/Current")).unwrap();
    symlink("Versions/Current/Helper", source.join("Helper")).unwrap();
    let mut plan = Plan::new(&Target::parse("aarch64-apple-darwin").unwrap());
    plan.add(
        &source,
        Path::new("Contents/Frameworks/Helper.framework"),
        false,
        &mut cache::Inputs::default(),
    )
    .unwrap();
    let output = root.path().join("output");
    plan.write(&output).unwrap();
    let deployed = output.join("Contents/Frameworks/Helper.framework");
    assert_eq!(
        fs::read_link(deployed.join("Versions/Current")).unwrap(),
        Path::new("A")
    );
    assert_eq!(
        fs::metadata(deployed.join("Helper"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o751
    );
}
