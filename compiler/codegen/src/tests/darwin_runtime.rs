use std::process::Command;

#[test]
fn macho_metadata_stays_in_its_own_executable_or_loaded_dylib() {
    let workspace = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .unwrap();
    let directory = tempfile::tempdir().unwrap();
    let library = directory.path().join("self image.dylib");
    let host = directory.path().join("host");
    for (shared, output) in [(true, &library), (false, &host)] {
        let mut command = Command::new("cc");
        command.args([
            "-std=c11", "-Wall", "-Wextra", "-Werror", "-O2", "-fPIC", "-pthread",
        ]);
        if shared {
            command.args(["-dynamiclib", "-DSCOOP_TEST_DYLIB=1"]);
        }
        for source in [
            "runtime/tests/darwin_image_test.c",
            "runtime/src/platform/image/macho.c",
            "runtime/src/image/ranges.c",
        ] {
            command.arg(workspace.join(source));
        }
        let compile = command.arg("-o").arg(output).output().unwrap();
        assert!(
            compile.status.success(),
            "{}",
            String::from_utf8_lossy(&compile.stderr)
        );
    }
    let output = Command::new(&host).arg(&library).output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        output.stdout,
        b"Mach-O executable and dylib self-image ranges passed\n"
    );
}
