//! Real PM preparation shared by driver tests that consume source Cone graphs.
#[path = "../../../tests/support/cargo_tools.rs"]
mod cargo_tools;

use std::{path::Path, process::Command};

use scoop_protocol::{CompilationPurpose, package::PreparedWorkspaceV1};

pub(crate) fn root(
    root: &Path,
    sysroot: &Path,
    purpose: CompilationPurpose,
) -> crate::BuildRootInput {
    let pm = cargo_tools::companion("scoop-pm");
    let invoke = |action: &str| {
        let output = Command::new(&pm)
            .arg(action)
            .arg(root)
            .arg("--offline")
            .arg("--sysroot")
            .arg(sysroot)
            .arg("--package-cache-dir")
            .arg(sysroot.with_extension("packages"))
            .arg("--purpose")
            .arg(match purpose {
                CompilationPurpose::Production => "production",
                CompilationPurpose::Test => "test",
            })
            .output()
            .expect("build the companion scoop-pm before driver tests");
        assert!(
            output.status.success(),
            "{action}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        output.stdout
    };
    invoke("resolve");
    let mut prepared: PreparedWorkspaceV1 = serde_json::from_slice(&invoke("metadata")).unwrap();
    prepared.validate().unwrap();
    assert_eq!(prepared.resolutions.len(), 1);
    crate::BuildRootInput::prepared(prepared.resolutions.remove(0)).unwrap()
}
