use super::*;
use crate::test_support;

#[test]
fn relocated_runtime_index_links_the_copied_objects() {
    let fixture = test_support::native_fixture();
    let directory = fixture.directory.path().join("relocated runtime");
    std::fs::create_dir(&directory).unwrap();
    let source = fixture.runtime.input_paths()[0].parent().unwrap();
    std::fs::copy(source.join("index.cbor"), directory.join("index.cbor")).unwrap();
    for object in fixture.runtime.objects() {
        let destination = directory.join(object.path().strip_prefix(source).unwrap());
        std::fs::create_dir_all(destination.parent().unwrap()).unwrap();
        std::fs::copy(object.path(), destination).unwrap();
    }
    let runtime = crate::RuntimeObjectSet::read_index(
        &directory.join("index.cbor"),
        fixture.profile.target(),
        fixture.profile.startup_toolchain().profile(),
    )
    .unwrap();
    assert!(
        runtime
            .objects()
            .iter()
            .all(|object| object.path().starts_with(&directory))
    );
    let output = link_program(
        &fixture.closure,
        &runtime,
        &fixture.profile,
        crate::ProgramLinkOptions {
            keep_symbols: true,
            emit_debug_file: false,
            enable_stack_trace: false,
        },
        &fixture.library_paths,
        &directory.join("program"),
        &directory.join("cache"),
    )
    .unwrap();
    assert_eq!(run(&output.path), b"42\n");
}

#[test]
#[cfg(unix)]
fn native_alias_link_tracks_real_input_updates_and_missing_input() {
    let fixture = test_support::native_fixture();
    let directory = fixture.directory.path();
    let object = directory.join("m23_final.o");
    let alias_directory = directory.join("alias");
    std::fs::create_dir(&alias_directory).unwrap();
    let alias = alias_directory.join("m23_final.o");
    std::os::unix::fs::symlink(&object, &alias).unwrap();
    let quoted_directory = directory.join("objects with spaces, 'single' and \"double\" \\");
    std::fs::create_dir(&quoted_directory).unwrap();
    let output = link_program(
        &fixture.closure,
        &fixture.runtime,
        &fixture.profile,
        crate::ProgramLinkOptions {
            keep_symbols: true,
            emit_debug_file: false,
            enable_stack_trace: false,
        },
        std::slice::from_ref(&alias_directory),
        &quoted_directory.join("program"),
        &quoted_directory.join("cache"),
    )
    .unwrap();
    assert_eq!(output.fingerprint, fixture.output.fingerprint);
    assert_eq!(output.plan_dump, fixture.output.plan_dump);
    assert_eq!(run(&output.path), b"42\n");

    let replacement = directory.join("replacement.c");
    std::fs::write(
        &replacement,
        "int m23_final(int value) { return value + 80; }\n",
    )
    .unwrap();
    test_support::compile_native(&fixture.profile, &replacement, &object);

    let changed = link_program(
        &fixture.closure,
        &fixture.runtime,
        &fixture.profile,
        crate::ProgramLinkOptions {
            keep_symbols: true,
            emit_debug_file: false,
            enable_stack_trace: false,
        },
        &fixture.library_paths,
        &directory.join("changed"),
        &quoted_directory.join("cache"),
    )
    .unwrap();
    assert_ne!(changed.fingerprint, output.fingerprint);
    assert_eq!(run(&changed.path), b"99\n");

    let old = std::fs::read(&output.path).unwrap();
    std::fs::remove_file(&alias).unwrap();
    let message = link_program(
        &fixture.closure,
        &fixture.runtime,
        &fixture.profile,
        crate::ProgramLinkOptions {
            keep_symbols: true,
            emit_debug_file: false,
            enable_stack_trace: false,
        },
        &[alias_directory],
        &output.path,
        &quoted_directory.join("cache"),
    )
    .err()
    .unwrap()
    .to_string();
    assert!(message.contains("m23_final"), "{message}");
    assert_eq!(std::fs::read(&output.path).unwrap(), old);
}

fn run(path: &Path) -> Vec<u8> {
    let output = std::process::Command::new(path)
        .env_clear()
        .output()
        .unwrap();
    assert!(output.status.success());
    output.stdout
}

#[test]
fn final_link_strips_internal_names_and_keeps_runtime_data() {
    use object::{Object, ObjectSection, ObjectSymbol};
    use std::collections::BTreeSet;

    let fixture = test_support::native_fixture();
    let stripped = link_program(
        &fixture.closure,
        &fixture.runtime,
        &fixture.profile,
        crate::ProgramLinkOptions {
            keep_symbols: false,
            emit_debug_file: false,
            enable_stack_trace: false,
        },
        &fixture.library_paths,
        &fixture.directory.path().join("stripped"),
        &fixture.directory.path().join("cache"),
    )
    .unwrap();
    let kept_bytes = std::fs::read(&fixture.output.path).unwrap();
    let stripped_bytes = std::fs::read(&stripped.path).unwrap();
    let kept = object::File::parse(kept_bytes.as_slice()).unwrap();
    let removed = object::File::parse(stripped_bytes.as_slice()).unwrap();
    let scoop_symbol = |symbol: &str| symbol.trim_start_matches('_').starts_with("scoop$1$");
    assert!(
        kept.symbols()
            .any(|symbol| symbol.name().is_ok_and(scoop_symbol))
    );
    assert!(
        !removed
            .symbols()
            .any(|symbol| symbol.name().is_ok_and(scoop_symbol))
    );
    assert!(stripped_bytes.len() < kept_bytes.len());
    assert_ne!(stripped.fingerprint, fixture.output.fingerprint);

    let imports = |file: &object::File<'_>| {
        file.imports()
            .unwrap()
            .into_iter()
            .map(|import| (import.library().to_vec(), import.name().to_vec()))
            .collect::<BTreeSet<_>>()
    };
    let exports = |file: &object::File<'_>| {
        file.exports()
            .unwrap()
            .into_iter()
            .map(|export| (export.name().to_vec(), export.address()))
            .collect::<BTreeSet<_>>()
    };
    assert_eq!(imports(&kept), imports(&removed));
    assert_eq!(exports(&kept), exports(&removed));
    for section in kept.sections().filter(|section| {
        matches!(
            section.name().unwrap(),
            ".text"
                | "__text"
                | ".llvm_stackmaps"
                | "__llvm_stackmaps"
                | ".eh_frame"
                | "__eh_frame"
                | "__unwind_info"
                | ".gcc_except_table"
                | "__gcc_except_tab"
        )
    }) {
        let name = section.name().unwrap();
        let other = removed.section_by_name(name).unwrap();
        assert_eq!(section.address(), other.address(), "{name}");
        assert_eq!(section.data().unwrap(), other.data().unwrap(), "{name}");
    }
    assert_eq!(run(&stripped.path), b"42\n");
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[test]
fn elf_shared_library_link_tracks_real_symbol_version_updates() {
    let fixture = test_support::native_fixture();
    let directory = fixture.directory.path();
    let libraries = directory.join("libraries");
    std::fs::create_dir(&libraries).unwrap();
    let library = libraries.join("libm23_final.so");
    let source = directory.join("shared.c");
    let script = directory.join("versions.map");
    let compile = |version: &str, offset: i32| {
        std::fs::write(
            &source,
            format!("int m23_final(int value) {{ return value + {offset}; }}\n"),
        )
        .unwrap();
        std::fs::write(
            &script,
            format!("{version} {{ global: m23_final; local: *; }};\n"),
        )
        .unwrap();
        let result = fixture
            .profile
            .startup_toolchain()
            .driver_command()
            .args(["-shared", "-fPIC", "-Wl,-soname,libm23_final.so"])
            .arg(format!("-Wl,--version-script={}", script.display()))
            .arg(&source)
            .arg("-o")
            .arg(&library)
            .scoop_output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    };
    compile("M28_1", 23);
    let link = || {
        link_program(
            &fixture.closure,
            &fixture.runtime,
            &fixture.profile,
            crate::ProgramLinkOptions {
                keep_symbols: true,
                emit_debug_file: false,
                enable_stack_trace: false,
            },
            std::slice::from_ref(&libraries),
            &directory.join("dso-program"),
            &directory.join("cache"),
        )
        .unwrap()
    };
    let first = link();
    assert_eq!(run(&first.path), b"42\n");
    compile("M28_2", 80);
    let second = link();
    assert_ne!(first.fingerprint, second.fingerprint);
    assert_eq!(run(&second.path), b"99\n");
}

#[cfg(target_os = "macos")]
#[test]
fn darwin_final_link_keeps_scoop_definitions_local() {
    use object::{Object, ObjectSymbol};
    let fixture = test_support::native_fixture();
    let bytes = std::fs::read(&fixture.output.path).unwrap();
    let file = object::File::parse(bytes.as_slice()).unwrap();
    let is_scoop_definition = |name: &str| name.starts_with("_scoop$1$");
    let definitions = file
        .symbols()
        .filter(|symbol| symbol.name().is_ok_and(is_scoop_definition))
        .collect::<Vec<_>>();
    assert!(!definitions.is_empty());
    assert!(definitions.iter().all(|symbol| symbol.is_local()));
    for kind in ["cb", "td"] {
        let prefix = format!("_scoop$1${kind}$");
        assert!(
            definitions
                .iter()
                .any(|symbol| symbol.name().unwrap().starts_with(&prefix)),
            "missing representative {kind} definition"
        );
    }
    for kind in ["bs", "be"] {
        let prefix = format!("_scoop$1${kind}$");
        assert!(
            definitions
                .iter()
                .all(|symbol| !symbol.name().unwrap().starts_with(&prefix)),
            "production {kind} aliases must not enter the final symbol table"
        );
    }
    let exports = file.exports().unwrap();
    assert!(
        exports
            .iter()
            .all(|symbol| { !is_scoop_definition(std::str::from_utf8(symbol.name()).unwrap()) })
    );
    assert!(exports.iter().any(|symbol| symbol.name() == b"_main"));
    assert!(exports.iter().any(|symbol| symbol.name() == b"_m23_final"));
    assert_eq!(run(&fixture.output.path), b"42\n");
}

#[cfg(target_os = "macos")]
#[test]
fn darwin_empty_code_labels_preserve_referenced_symbols_and_sections() {
    use object::{Object, ObjectSection, ObjectSymbol, RelocationTarget};
    let fixture = test_support::native_fixture();
    let directory = fixture.directory.path();
    let source = directory.join("empty-code.s");
    let object = directory.join("empty-code.o");
    std::fs::write(
        &source,
        ".section __TEXT,__text,regular,pure_instructions\n\
         unused_local:\n\
         used_local:\n\
         .globl _empty_public\n\
         _empty_public:\n\
         .section __TEXT,__code,regular,pure_instructions\n\
         live_local:\n\
         nop\n\
         .section __DATA,__data\n\
         .globl _references\n\
         _references:\n\
         .quad _references - used_local\n\
         .quad live_local\n",
    )
    .unwrap();
    let result = fixture
        .profile
        .startup_toolchain()
        .driver_command()
        .arg("-c")
        .arg(&source)
        .arg("-o")
        .arg(&object)
        .scoop_output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let bytes = std::fs::read(object).unwrap();
    for trailer in [b"".as_slice(), b"native producer trailer"] {
        let mut input = bytes.clone();
        input.extend(trailer);
        let before = object::File::parse(input.as_slice()).unwrap();
        let output = scoop_slib::remove_empty_macho_code_labels(&input).unwrap();
        let after = object::File::parse(output.as_slice()).unwrap();
        assert!(
            after
                .symbols()
                .all(|symbol| symbol.name().unwrap() != "unused_local")
        );
        for name in ["used_local", "_empty_public", "live_local", "_references"] {
            assert!(
                after.symbols().any(|symbol| symbol.name().unwrap() == name),
                "{name}"
            );
        }
        for (before_section, after_section) in before.sections().zip(after.sections()) {
            assert_eq!(
                before_section.name().unwrap(),
                after_section.name().unwrap()
            );
            assert_eq!(
                before_section.data().unwrap(),
                after_section.data().unwrap()
            );
            let targets = |file: &object::File<'_>, section: &object::Section<'_, '_>| {
                section
                    .relocations()
                    .map(|(offset, relocation)| {
                        let target = match relocation.target() {
                            RelocationTarget::Symbol(index) => file
                                .symbol_by_index(index)
                                .unwrap()
                                .name()
                                .unwrap()
                                .to_owned(),
                            RelocationTarget::Section(index) => file
                                .section_by_index(index)
                                .unwrap()
                                .name()
                                .unwrap()
                                .to_owned(),
                            RelocationTarget::Absolute => "absolute".to_owned(),
                            target => panic!("unexpected relocation target {target:?}"),
                        };
                        (offset, target)
                    })
                    .collect::<Vec<_>>()
            };
            assert_eq!(
                targets(&before, &before_section),
                targets(&after, &after_section)
            );
        }
    }
}
