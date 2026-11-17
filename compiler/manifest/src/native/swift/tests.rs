use super::*;
use crate::test_catalog::parse_cone_manifest;

const CONE: &str = "schema = 1\n[cone]\ngroup = 'swift.test'\nname = 'client'\nversion = '0.1.0'\nkind = 'library'\n";
const LIBRARY: &str = "\n[[native.swift]]\npackage = '../native/swiftmath'\nproduct = 'SwiftMath'\nlinkage = 'static'\nwhen = { os = 'darwin' }\n";

#[test]
fn swift_recipes_project_only_their_logical_library_requirement() {
    let source =
        format!("{CONE}{LIBRARY}inputs = ['../config/input.json']\nenv = ['SWIFT_FEATURE']\n");
    let parsed = parse_cone_manifest(&source).unwrap();
    let native = parsed.semantic().native();
    let target = scoop_target_data::DARWIN_AARCH64_ID;
    let swift = native.swift_libraries(target).next().unwrap();
    assert_eq!(swift.package().as_str(), "../native/swiftmath");
    assert_eq!(swift.linkage(), SwiftLinkage::Static);
    assert_eq!(swift.env(), ["SWIFT_FEATURE"]);
    assert_eq!(swift.inputs()[0].as_str(), "../config/input.json");
    assert!(native.sources().is_empty());
    let projected = parsed.clone().into_compiler_projection();
    assert!(
        projected
            .semantic()
            .native()
            .swift_libraries(target)
            .next()
            .is_none()
    );
    assert_eq!(
        native.library_requirements(target).unwrap(),
        projected
            .semantic()
            .native()
            .library_requirements(target)
            .unwrap()
    );
    let changed = parse_cone_manifest(
        &source
            .replace("../native/swiftmath", "../another")
            .replace("SWIFT_FEATURE", "OTHER_FEATURE"),
    )
    .unwrap();
    assert_eq!(
        projected.semantic(),
        changed.into_compiler_projection().semantic()
    );
    assert!(source[swift.span()].contains("SwiftMath"));
}

#[test]
fn frameworks_keep_linkage_separate_from_container_kind() {
    for linkage in ["static", "dynamic"] {
        let source = format!(
            "{CONE}{}format = 'framework'\nbundle_id = 'dev.example.swiftmath'",
            LIBRARY.replace("static", linkage)
        );
        let parsed = parse_cone_manifest(&source).unwrap();
        let swift = parsed
            .semantic()
            .native()
            .swift_libraries(scoop_target_data::DARWIN_AARCH64_ID)
            .next()
            .unwrap();
        assert_eq!(swift.library_kind(), NativeLibraryKind::Framework);
        assert!(
            matches!(swift.format(), SwiftLibraryFormat::Framework { bundle_id } if bundle_id == "dev.example.swiftmath")
        );
    }
}

#[test]
fn swift_configuration_errors_are_reported_with_a_manifest_span() {
    for tail in [
        "format = 'framework'",
        "format = 'unknown'",
        "bundle_id = 'dev.example.invalid'",
        "env = ['A=B']",
        "env = ['REPEATED', 'REPEATED']",
        "inputs = ['some/*.json']",
        "format = 'framework'\nbundle_id = 'invalid/bundle'",
    ] {
        let error = parse_cone_manifest(&format!("{CONE}{LIBRARY}{tail}")).unwrap_err();
        assert!(matches!(
            error.kind(),
            ManifestParseErrorKind::InvalidNative(..)
        ));
        assert!(error.span().is_some());
    }
    for (from, to) in [
        ("'static'", "'automatic'"),
        ("'../native/swiftmath'", "'/absolute/package'"),
    ] {
        assert!(parse_cone_manifest(&format!("{CONE}{}", LIBRARY.replace(from, to))).is_err());
    }
}

#[test]
fn duplicate_and_incompatible_native_bindings_are_rejected() {
    assert!(parse_cone_manifest(&format!("{CONE}{LIBRARY}{LIBRARY}")).is_err());
    for kind in ["static", "dynamic", "framework", "default"] {
        let source = format!(
            "{CONE}{LIBRARY}\n[[native.libraries]]\nname = 'SwiftMath'\nkind = '{kind}'\nwhen = {{ os = 'darwin' }}"
        );
        let parsed = parse_cone_manifest(&source);
        assert_eq!(parsed.is_ok(), kind == "static");
        if let Ok(parsed) = parsed {
            assert_eq!(
                parsed
                    .semantic()
                    .native()
                    .library_requirements(scoop_target_data::DARWIN_AARCH64_ID)
                    .unwrap()
                    .len(),
                1
            );
        }
    }
}

#[test]
fn unmatched_swift_declarations_do_not_create_native_requirements() {
    let parsed = parse_cone_manifest(&format!(
        "{CONE}{}",
        LIBRARY.replace("os = 'darwin'", "os = 'darwin', arch = 'x86_64'")
    ))
    .unwrap();
    assert!(
        parsed
            .semantic()
            .native()
            .swift_libraries(scoop_target_data::DARWIN_AARCH64_ID)
            .next()
            .is_none()
    );
    assert!(
        parsed
            .semantic()
            .native()
            .library_requirements(scoop_target_data::DARWIN_AARCH64_ID)
            .unwrap()
            .is_empty()
    );
}
