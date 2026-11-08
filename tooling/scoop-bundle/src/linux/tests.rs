use super::packages;
use crate::config;
use std::{collections::BTreeMap, fs, path::Path};

fn settings(root: &Path, extra: &str) -> config::Loaded {
    let file = root.join("Bundle.toml");
    fs::write(&file,format!("[bundle]\nschema=1\nidentifier='dev.example.app'\nname='example-app'\nversion='1.2.3'\ndescription='Example application'\nlicense='MIT'\n{extra}")).unwrap();
    config::Loaded::finish(
        config::parse_external(&file).unwrap(),
        file,
        None,
        None,
        &BTreeMap::new(),
    )
    .unwrap()
}

#[test]
fn deb_metadata_keeps_declared_dependencies_and_rejects_header_injection() {
    let temporary = tempfile::tempdir().unwrap();
    let value = settings(
        temporary.path(),
        "[bundle.linux.deb]\nmaintainer='Example <dev@example.com>'\ndepends=['libgtk-4-1 (>= 4.18)', 'libadwaita-1-0 (>= 1.4)']\n",
    );
    let text = packages::deb_control(&value).unwrap();
    assert!(text.contains("Version: 1.2.3-1\n"));
    assert!(text.contains("Depends: libgtk-4-1 (>= 4.18), libadwaita-1-0 (>= 1.4)\n"));
    let mut invalid = value;
    invalid.config.linux.deb.maintainer = Some("Name\nDepends: another-package".into());
    assert!(packages::deb_control(&invalid).is_err());
    invalid.config.linux.deb.maintainer = Some("Name <dev@example.com>".into());
    invalid.version = "1.2.3-beta.1".parse().unwrap();
    assert!(packages::deb_control(&invalid).is_err());
    invalid.config.linux.deb.version = Some("1.2.3~beta.1".into());
    assert!(
        packages::deb_control(&invalid)
            .unwrap()
            .contains("1.2.3~beta.1-1")
    );
}

#[test]
fn desktop_actions_preserve_order_and_literal_arguments() {
    let temporary = tempfile::tempdir().unwrap();
    let value = settings(
        temporary.path(),
        r#"
[[bundle.linux.actions]]
id = 'new-window'
title = '新建窗口'
arguments = ['--new-window', '', 'a b', '100%', '$HOME', 'quote"', 'back\slash']
[[bundle.linux.actions]]
id = 'preferences'
title = 'Preferences'
arguments = ['--preferences']
"#,
    );
    let text = super::desktop(&value, false, true).unwrap();
    assert!(text.contains("StartupWMClass=dev.example.app\nActions=new-window;preferences;\n"));
    assert!(text.contains("[Desktop Action new-window]\nName=新建窗口\n"));
    assert!(text.contains(r#"Exec="/usr/bin/example-app" "--new-window" "" "a b" "100%%" "\\$HOME" "quote\\"" "back\\\\slash""#), "{text}");
    assert!(text.contains("[Desktop Action preferences]\nName=Preferences\nExec=\"/usr/bin/example-app\" \"--preferences\"\nIcon=dev.example.app\n"));
    let appimage = super::desktop(&value, true, false).unwrap();
    assert!(appimage.contains("Exec=\"example-app\" \"--preferences\"\n"));
    assert!(!appimage.contains("Icon="));

    let mut invalid = value;
    invalid.config.linux.actions[1].id = "new-window".into();
    assert!(super::desktop(&invalid, false, false).is_err());
    invalid.config.linux.actions[1].id = "bad]\nExec=other".into();
    assert!(super::desktop(&invalid, false, false).is_err());
    invalid.config.linux.actions[1].id = "valid".into();
    invalid.config.linux.actions[1]
        .arguments
        .push("nul\0value".into());
    assert!(super::desktop(&invalid, false, false).is_err());
}

#[cfg(unix)]
#[test]
fn rpm_payload_contains_relative_symlinks_empty_directories_and_native_metadata() {
    use std::os::unix::fs::symlink;
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path();
    let value = settings(root, "[bundle.linux.rpm]\nrequires=['gtk4 >= 4.18']\n");
    let payload = root.join("payload");
    fs::create_dir_all(payload.join("usr/bin")).unwrap();
    fs::create_dir_all(payload.join("usr/lib/dev.example.app/bin")).unwrap();
    fs::create_dir_all(payload.join("usr/share/dev.example.app/empty")).unwrap();
    fs::write(
        payload.join("usr/lib/dev.example.app/bin/example-app"),
        b"program",
    )
    .unwrap();
    symlink(
        "../lib/dev.example.app/bin/example-app",
        payload.join("usr/bin/example-app"),
    )
    .unwrap();
    let artifact = packages::rpm(&value, &payload, root).unwrap();
    let package = rpm::Package::open(root.join(artifact)).unwrap();
    assert_eq!(package.metadata.get_name().unwrap(), "example-app");
    assert_eq!(package.metadata.get_version().unwrap(), "1.2.3");
    assert!(
        package
            .metadata
            .get_requires()
            .unwrap()
            .contains(&rpm::Dependency::greater_eq("gtk4", "4.18"))
    );
    let entries = package.metadata.get_file_entries().unwrap();
    assert!(
        entries
            .iter()
            .any(|entry| entry.path() == Path::new("/usr/share/dev.example.app/empty"))
    );
    let link = entries
        .iter()
        .find(|entry| entry.path() == Path::new("/usr/bin/example-app"))
        .unwrap();
    assert_eq!(
        link.linkto(),
        Some("../lib/dev.example.app/bin/example-app")
    );
}
