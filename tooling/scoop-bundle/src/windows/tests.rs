use std::{collections::BTreeMap, path::Path};

use super::{msi, package_version};
use crate::{config::Loaded, deploy::Plan, target::Target};

fn settings(version: &str, extra: &str) -> Loaded {
    let table: toml::Table = format!(
        r#"
schema = 1
identifier = "dev.scoop.windows-test"
name = "test-app"
version = "{version}"
{extra}
"#
    )
    .parse()
    .unwrap();
    Loaded::finish(
        table.try_into().unwrap(),
        Path::new("/project/Bundle.toml").into(),
        None,
        None,
        &BTreeMap::new(),
    )
    .unwrap()
}

#[test]
fn installer_versions_require_explicit_lossy_mappings_and_native_ranges() {
    assert_eq!(
        package_version(&settings("1.2.3", ""), "nsis").unwrap(),
        "1.2.3.0"
    );
    assert_eq!(
        package_version(&settings("1.2.3", ""), "msi").unwrap(),
        "1.2.3"
    );
    assert!(package_version(&settings("256.0.0", ""), "msi").is_err());
    assert!(package_version(&settings("65536.0.0", ""), "nsis").is_err());
    assert!(package_version(&settings("1.2.3-rc.1", ""), "msi").is_err());
    let explicit = settings(
        "1.2.3-rc.1",
        "[windows.msi]\nproduct-version = '1.2.2'\n[windows.nsis]\nfile-version = '1.2.2.1'",
    );
    assert_eq!(package_version(&explicit, "msi").unwrap(), "1.2.2");
    assert_eq!(package_version(&explicit, "nsis").unwrap(), "1.2.2.1");
}

#[test]
fn component_identity_is_stable_for_windows_paths_and_scoped_to_the_product() {
    let first = settings(
        "1.2.3",
        "[windows.msi]\nupgrade-code = 'e5e230c7-6b22-44f6-9e7c-094034203645'",
    );
    let next = settings(
        "1.2.4",
        "[windows.msi]\nupgrade-code = 'e5e230c7-6b22-44f6-9e7c-094034203645'",
    );
    let component = |settings, user, path: &str| {
        msi::identity(
            msi::upgrade_code(settings).unwrap(),
            user,
            "file",
            Path::new(path),
        )
        .unwrap()
    };
    let id = component(&first, true, "resources/A.txt");
    assert_eq!(id, component(&next, true, "RESOURCES/a.txt"));
    assert_ne!(id, component(&first, false, "resources/A.txt"));
    assert_ne!(id, component(&first, true, "resources/B.txt"));
    assert!(msi::upgrade_code(&settings("1.2.3", "")).is_err());
}

#[test]
fn windows_payload_rejects_reserved_names_and_generated_uninstaller_collisions() {
    let target = Target::parse("x86_64-pc-windows-msvc-ucrt").unwrap();
    let mut plan = Plan::new(&target);
    for name in ["CON", "resources/LPT1.txt", "resource.txt.", "a?b"] {
        assert!(plan.generated(name, Vec::new()).is_err(), "{name}");
    }
    plan.generated("Uninstall.EXE", Vec::new()).unwrap();
    assert!(plan.require_unused(Path::new("uninstall.exe")).is_err());
}
