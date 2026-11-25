use super::*;

#[test]
fn parallel_translation_units_preserve_object_bytes_order_and_input_observations() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("cone");
    let manifest = multiple_sources(&root);
    let prepare = |jobs| {
        prepare_native_objects(
            &manifest,
            target().lir_target(),
            || Ok(target()),
            OptimizationMode::Debug,
            &crate::development_runtime_root(),
            &directory.path().join(format!("cache-{jobs}")),
            scoop_protocol::CompilationPurpose::Production,
            std::num::NonZeroUsize::new(jobs).unwrap(),
        )
        .unwrap()
    };
    let serial = prepare(1);
    let parallel = prepare(4);
    assert_eq!(serial.objects().len(), 3);
    assert_eq!(serial.key(), parallel.key());
    assert_eq!(serial.input_metadata(), parallel.input_metadata());
    for (expected, actual) in serial.objects().iter().zip(parallel.objects()) {
        assert_eq!(
            std::fs::read(expected).unwrap(),
            std::fs::read(actual).unwrap()
        );
    }
    assert_eq!(prepare(4), parallel);
}

#[test]
fn parallel_failures_report_the_first_source_and_leave_no_partial_outputs() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("cone");
    let manifest = multiple_sources(&root);
    std::fs::write(root.join("answer.c"), "#error first source failed\n").unwrap();
    std::fs::write(root.join("last.c"), "#error later source failed\n").unwrap();
    let mut failures = Vec::new();
    for jobs in [1, 4] {
        let cache = directory.path().join(format!("cache-{jobs}"));
        let failure = prepare_native_objects(
            &manifest,
            target().lir_target(),
            || Ok(target()),
            OptimizationMode::Debug,
            &crate::development_runtime_root(),
            &cache,
            scoop_protocol::CompilationPurpose::Production,
            std::num::NonZeroUsize::new(jobs).unwrap(),
        )
        .unwrap_err();
        failures.push((failure.code(), failure.to_string()));
        assert!(
            std::fs::read_dir(cache.join("native-objects-v3"))
                .unwrap()
                .all(|entry| {
                    let path = entry.unwrap().path();
                    path.extension()
                        .is_some_and(|extension| extension == "lock")
                })
        );
    }
    assert_eq!(failures[0], failures[1]);
    assert!(failures[0].1.contains("first source failed"));
    assert!(!failures[0].1.contains("later source failed"));
}

fn multiple_sources(root: &Path) -> LoadedConeManifest {
    manifest(
        root,
        "#include <value.h>\nint answer(void);\nint answer(void) { return VALUE; }\n",
    );
    std::fs::write(root.join("include/value.h"), "#define VALUE 41\n").unwrap();
    std::fs::write(
        root.join("last.c"),
        "int last(void);\nint last(void) { return 3; }\n",
    )
    .unwrap();
    std::fs::write(
        root.join("middle.c"),
        "int middle(void);\nint middle(void) { return 2; }\n",
    )
    .unwrap();
    let path = root.join("Cone.toml");
    let mut text = std::fs::read_to_string(&path).unwrap();
    // Deliberately declare these in reverse order.
    text.push_str(
        "\n[[native.sources]]\npath = \"last.c\"\n[[native.sources]]\npath = \"middle.c\"\n",
    );
    std::fs::write(path, text).unwrap();
    load_cone_manifest(&ManifestRootLocator::cone_directory(root)).unwrap()
}

#[test]
fn object_records_reuse_outputs_and_follow_actual_header_changes() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("cone");
    let manifest = manifest(
        &root,
        "#include <value.h>\nint answer(void);\nint answer(void) { return VALUE; }\n",
    );
    std::fs::write(root.join("include/value.h"), "#define VALUE 41\n").unwrap();
    let cache = directory.path().join("cache");
    let resolutions = std::cell::Cell::new(0);
    let prepare = || {
        prepare_native_objects(
            &manifest,
            target().lir_target(),
            || {
                resolutions.set(resolutions.get() + 1);
                Ok(target())
            },
            OptimizationMode::Debug,
            &crate::development_runtime_root(),
            &cache,
            scoop_protocol::CompilationPurpose::Production,
            std::num::NonZeroUsize::new(1).unwrap(),
        )
        .unwrap()
    };
    let cold = prepare();
    let files = std::fs::read_dir(cold.objects()[0].parent().unwrap())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<_>>();
    assert_eq!(files, cold.objects());
    let before = cold.objects()[0].metadata().unwrap().modified().unwrap();
    assert_eq!(prepare(), cold);
    assert_eq!(resolutions.get(), 1);
    std::fs::write(root.join("program.scoop"), "fun value(): Int = 1\n").unwrap();
    assert_eq!(prepare(), cold);
    std::fs::write(root.join("program.scoop"), "fun value(): Int = 2\n").unwrap();
    assert_eq!(prepare(), cold);
    assert_eq!(
        cold.objects()[0].metadata().unwrap().modified().unwrap(),
        before
    );
    std::fs::write(root.join("include/value.h"), "#define VALUE 42\n").unwrap();
    let changed = prepare();
    assert_eq!(resolutions.get(), 2);
    assert_ne!(cold.key(), changed.key());
    assert_ne!(cold.objects(), changed.objects());
    assert_ne!(
        std::fs::read(&cold.objects()[0]).unwrap(),
        std::fs::read(&changed.objects()[0]).unwrap()
    );
    assert_eq!(prepare(), changed);
}

#[test]
fn quoted_macro_include_observes_its_own_directory() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("cone");
    let manifest = manifest(
        &root,
        "#include <nested/entry.h>\nint answer(void);\nint answer(void) { return VALUE; }\n",
    );
    std::fs::create_dir_all(root.join("include/nested")).unwrap();
    std::fs::write(
        root.join("include/nested/entry.h"),
        "#define HEADER \"value.h\"\n#include HEADER\n",
    )
    .unwrap();
    std::fs::write(root.join("include/value.h"), "#define VALUE 1\n").unwrap();
    let cache = directory.path().join("cache");
    let prepare = || {
        prepare_native_objects(
            &manifest,
            target().lir_target(),
            || Ok(target()),
            OptimizationMode::Debug,
            &crate::development_runtime_root(),
            &cache,
            scoop_protocol::CompilationPurpose::Production,
            std::num::NonZeroUsize::new(1).unwrap(),
        )
        .unwrap()
    };
    let cold = prepare();
    std::fs::write(root.join("include/nested/unrelated.h"), "").unwrap();
    assert_eq!(prepare(), cold);
    std::fs::write(root.join("include/nested/value.h"), "#define VALUE 2\n").unwrap();
    let changed = prepare();
    assert_ne!(changed.key(), cold.key());
    assert_ne!(
        std::fs::read(&cold.objects()[0]).unwrap(),
        std::fs::read(&changed.objects()[0]).unwrap()
    );
    assert_eq!(prepare(), changed);
}

#[test]
fn native_compilation_failure_does_not_publish_a_record() {
    let directory = tempfile::tempdir().unwrap();
    let manifest = manifest(&directory.path().join("cone"), "this is not valid C;\n");
    let cache = directory.path().join("cache");
    assert!(
        prepare_native_objects(
            &manifest,
            target().lir_target(),
            || Ok(target()),
            OptimizationMode::Debug,
            &crate::development_runtime_root(),
            &cache,
            scoop_protocol::CompilationPurpose::Production,
            std::num::NonZeroUsize::new(1).unwrap(),
        )
        .is_err()
    );
    assert!(
        std::fs::read_dir(cache.join("native-objects-v3"))
            .unwrap()
            .all(|entry| entry
                .unwrap()
                .path()
                .extension()
                .is_none_or(|e| e != "json"))
    );
}
