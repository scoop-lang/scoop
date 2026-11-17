use super::*;

#[test]
fn clang_const_metadata_is_read_only_data() {
    let profile =
        scoop_toolchain::ValidatedFinalLinkProfile::resolve("aarch64-apple-darwin").unwrap();
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("metadata.c");
    let object = directory.path().join("metadata.o");
    std::fs::write(&source, "__attribute__((used, section(\"__DATA_CONST,__const\"))) const unsigned long metadata = 11;\n").unwrap();
    crate::test_support::compile_native(&profile, &source, &object);
    let bytes = std::fs::read(object).unwrap();
    let info = NativeObjectInfo::read_with_toolchain(&bytes, profile.startup_toolchain().profile())
        .unwrap();
    let definition = info.definitions["_metadata"];
    assert_eq!(definition.kind, NativeSymbolKind::Data);
    assert!(definition.read_only && definition.binding == NativeSymbolBinding::Strong);
}

#[test]
fn zippered_objects_and_dylibs_select_the_macos_deployment() {
    let profile =
        scoop_toolchain::ValidatedFinalLinkProfile::resolve("aarch64-apple-darwin").unwrap();
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("answer.c");
    std::fs::write(&source, "int answer(void) { return 42; }\n").unwrap();
    for (name, target, variant, accepted) in [
        (
            "zippered",
            "arm64-apple-macosx11.0",
            Some("arm64-apple-ios99.0-macabi"),
            true,
        ),
        ("catalyst", "arm64-apple-ios14.0-macabi", None, false),
        (
            "newer",
            "arm64-apple-macosx99.0",
            Some("arm64-apple-ios14.0-macabi"),
            false,
        ),
    ] {
        for (extension, mode, kind) in [
            ("o", "-c", crate::native_input::NativeFileKind::Object),
            (
                "dylib",
                "-dynamiclib",
                crate::native_input::NativeFileKind::Dylib,
            ),
        ] {
            let path = directory.path().join(format!("{name}.{extension}"));
            let mut command = std::process::Command::new("/usr/bin/xcrun");
            command.args(["clang", "-target", target, mode]);
            if let Some(variant) = variant {
                command.args(["-darwin-target-variant", variant]);
            }
            let output = command.arg(&source).arg("-o").arg(&path).output().unwrap();
            assert!(
                output.status.success(),
                "{command:?}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            let result = crate::native_input::locate::read(&path, kind, &profile, false);
            if accepted {
                assert!(
                    result.is_ok(),
                    "{name}.{extension}: {}",
                    result.err().unwrap()
                );
            } else {
                let error = result.err().expect("incompatible deployment accepted");
                assert_eq!(error.code(), scoop_source::DiagnosticCode::E2305);
                assert!(
                    error.to_string().contains("macOS") || error.to_string().contains("deployment")
                );
            }
        }
    }
}
