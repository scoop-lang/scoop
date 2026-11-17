use super::*;
use crate::test_support;
use scoop_manifest::{SwiftLibraryFormat, SwiftLinkage};
use scoop_toolchain::{NativeLibraryOutput, NativeLinkSettings};
use std::process::Command;

#[test]
fn exact_archive_and_framework_follow_only_selected_member_autolinks() {
    let fixture = test_support::native_fixture();
    let root = fixture.directory.path().join("produced");
    std::fs::create_dir(&root).unwrap();
    let sources = [
        (
            "entry",
            "extern int a(void), z(void); int m23_final(int x) { return a() + z(); }",
        ),
        (
            "late",
            "__asm__(\".linker_option \\\"-lChain\\\"\"); int z(void) { return 3; }",
        ),
        ("helper", "int a(void) { return 40; }"),
        (
            "unused",
            "__asm__(\".linker_option \\\"-lUnusedDependency\\\"\"); int unused(void) { return 1; }",
        ),
    ];
    for (name, source) in sources {
        let path = root.join(format!("{name}.c"));
        std::fs::write(&path, source).unwrap();
        test_support::compile_native(&fixture.profile, &path, &root.join(format!("{name}.o")));
    }
    let archive = root.join("selected.a");
    for (path, members) in [
        (archive.clone(), vec!["entry", "late", "unused"]),
        (root.join("libChain.a"), vec!["helper"]),
    ] {
        let result = Command::new("/usr/bin/ar")
            .arg("rcs")
            .arg(&path)
            .args(members.iter().map(|name| root.join(format!("{name}.o"))))
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    let mut output = NativeLibraryOutput {
        name: "m23_final".into(),
        linkage: SwiftLinkage::Static,
        format: SwiftLibraryFormat::Library,
        library: archive.clone(),
        binary: archive,
        dependencies: Vec::new(),
        resources: Vec::new(),
        settings: NativeLinkSettings::default(),
    };
    for framework in [false, true] {
        if framework {
            let path = root.join("m23_final.framework");
            let binary = path.join("Versions/A/m23_final");
            std::fs::create_dir_all(binary.parent().unwrap()).unwrap();
            std::fs::copy(&output.binary, &binary).unwrap();
            output.format = SwiftLibraryFormat::Framework {
                bundle_id: "dev.scoop.test".into(),
            };
            output.library = path;
            output.binary = binary;
        }
        let mut roots = fixture.library_paths.clone();
        if framework {
            let mut relative = output.clone();
            relative.library = relative.library.strip_prefix(&root).unwrap().to_owned();
            relative.binary = relative.binary.strip_prefix(&root).unwrap().to_owned();
            let mut unused = relative.clone();
            unused.name = "UnselectedProduct".into();
            unused.library = "libraries/absent.framework".into();
            unused.binary = "libraries/absent.framework/absent".into();
            unused
                .settings
                .libraries
                .insert("UnselectedMissingLibrary".into());
            std::fs::write(
                root.join("native-outputs.json"),
                serde_json::to_vec(&scoop_toolchain::native_output::NativeOutputsDescriptor {
                    schema: 1,
                    outputs: vec![relative, unused],
                })
                .unwrap(),
            )
            .unwrap();
            roots.push(root.clone());
        }
        let linked = link_program(
            &fixture.closure,
            &fixture.runtime,
            &fixture.profile,
            ProgramLinkOptions {
                native_outputs: if framework {
                    &[]
                } else {
                    std::slice::from_ref(&output)
                },
                ..Default::default()
            },
            &roots,
            &root.join(if framework {
                "framework-program"
            } else {
                "archive-program"
            }),
            &root.join("cache"),
        )
        .unwrap();
        // The explicit root still contains the older m23_final.o returning 42.
        let run = Command::new(&linked.path).output().unwrap();
        assert!(run.status.success());
        assert_eq!(run.stdout, b"43\n");
        assert!(linked.plan_dump.contains("Chain"));
        assert!(!linked.plan_dump.contains("UnusedDependency"));
        assert!(!linked.plan_dump.contains("UnselectedMissingLibrary"));
        assert_eq!(linked.native_outputs.len(), 1);
        assert!(
            linked
                .input_paths
                .contains(&output.binary.canonicalize().unwrap())
        );
    }
}
