use super::*;

mod independence;

#[test]
fn actual_hir_type_uses_drive_mir_dependency_projection() {
    let target = resolved_target().expect("supported host target");
    let sysroot = tempfile::tempdir().unwrap();
    let core = bootstrap_core(sysroot.path(), &target);
    let bytes = std::fs::read(core.artifact().path()).unwrap();
    let fixtures = crate::workspace_root().join("tests/fixtures/language/materialized-type-uses");
    for (name, required) in [
        ("standalone", &["Boolean", "ExecutorTask", "Long"][..]),
        ("combined", &["Any", "Boolean", "String"][..]),
        ("initialization", &["Boolean", "String", "Unit"][..]),
    ] {
        let source = std::fs::read_to_string(fixtures.join(format!("{name}.scoop")))
            .unwrap()
            .replacen("package test\n", "package dev.example\n", 1);
        support::with_inspection(
            sysroot.path(),
            &target,
            &bytes,
            &source,
            |input, projection| {
                if name == "standalone" {
                    independence::check(input);
                }
                let expected = input
                    .public
                    .external_references()
                    .materialized_type_dependencies(input.mir.module().cone, input.identities)
                    .unwrap();
                let mut actual = Vec::new();
                let mut names = std::collections::BTreeSet::new();
                for usage in projection {
                    let exact = match usage.target() {
                        mir::MirTypeBridgeTargetV1::Type(exact) => exact,
                        // Ordinary methods can add callable dependencies.
                        mir::MirTypeBridgeTargetV1::Callable(_) => continue,
                        _ => panic!("fixture requires external types and their value methods"),
                    };
                    let key = input
                        .identities
                        .canonical_key::<_, scoop_identity::ExactTypeKey>(exact)
                        .unwrap();
                    let scoop_identity::ExactTypeKey::Nominal(owner) = key.as_ref() else {
                        panic!("parameter-free nominal")
                    };
                    let declaration = input
                        .identities
                        .canonical_key::<_, scoop_identity::SourceDeclarationKey>(*owner)
                        .unwrap();
                    assert_eq!(usage.provider(), declaration.origin());
                    let scoop_identity::DeclarationName::Named(name) = declaration.name() else {
                        panic!("fixture nominal name")
                    };
                    actual.push((usage.provider(), exact));
                    names.insert(name.as_str().to_owned());
                }
                actual.sort_unstable();
                assert_eq!(actual, expected, "{name} HIR to MIR type dependencies");
                for dependency in required {
                    assert!(names.contains(*dependency), "{name} requires {dependency}");
                }
            },
            |_, _| {},
        );
    }
}
