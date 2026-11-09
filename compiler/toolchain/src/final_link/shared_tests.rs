use super::*;
use object::read::elf::ProgramHeader;
use object::{Object, ObjectSection};
use scoop_process::CommandExt;

#[path = "shared/darwin.rs"]
mod darwin_support;

fn run(command: &mut Command) -> std::process::Output {
    let output = command.scoop_output().unwrap();
    assert!(
        output.status.success(),
        "{command:?}: {}\n{}{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn system_libraries(profile: &ValidatedFinalLinkProfile, command: &mut Command, directory: &Path) {
    match profile {
        ValidatedFinalLinkProfile::Darwin(profile) => {
            darwin_support::append_libraries(profile, command, directory);
        }
        ValidatedFinalLinkProfile::Linux(profile) => profile.append_system_libraries(command),
        ValidatedFinalLinkProfile::Windows(_) => panic!("Unix test profile"),
    }
}

#[test]
fn shared_libraries_load_with_hidden_callbacks_and_existing_thread_tls() {
    #[cfg(target_os = "macos")]
    let cases = [
        ("aarch64-apple-darwin", None, false),
        ("x86_64-apple-darwin", None, false),
        ("aarch64-apple-darwin", None, true),
        ("x86_64-apple-darwin", None, true),
    ];
    #[cfg(target_os = "linux")]
    let cases = [
        ("x86_64-unknown-linux-gnu", None, false),
        (
            "x86_64-unknown-linux-gnu",
            Some(PathBuf::from("clang")),
            false,
        ),
        ("x86_64-unknown-linux-musl", None, false),
        ("x86_64-unknown-linux-gnu", None, true),
        (
            "x86_64-unknown-linux-gnu",
            Some(PathBuf::from("clang")),
            true,
        ),
    ];
    for (triple, compiler, cxx) in cases {
        let target = ResolvedTargetProfile::resolve_with(
            triple,
            &CToolchainOptions {
                compiler,
                ..Default::default()
            },
        )
        .unwrap();
        let directory = tempfile::Builder::new()
            .prefix("scoop shared , ")
            .tempdir()
            .unwrap();
        let executable = target
            .final_link_with_cxx(
                &FinalLinkOptions {
                    mode: triple.ends_with("musl").then_some(LinkMode::Dynamic),
                    ..Default::default()
                },
                false,
            )
            .unwrap();
        let source = directory.path().join("library.c");
        let object = directory.path().join("library.o");
        std::fs::write(&source, include_str!("shared/library.c")).unwrap();
        let mut compile = target
            .c_bridge_toolchain()
            .object_compilation_command(&source, &object);
        compile.args(["-funwind-tables", "-fno-omit-frame-pointer"]);
        if cxx {
            compile.arg("-DSCOOP_TEST_CPP");
        }
        if let ValidatedFinalLinkProfile::Linux(profile) = &executable {
            compile
                .arg("-I")
                .arg(profile.unwind_prefix().unwrap().join("include"));
        }
        run(&mut compile);
        let cpp_object = cxx.then(|| {
            let cpp = crate::ValidatedCxxToolchain::resolve_with(target.c_bridge_toolchain(), None)
                .unwrap();
            let source = directory.path().join("library.cpp");
            let object = directory.path().join("library-cpp.o");
            std::fs::write(&source, include_str!("shared/library.cpp")).unwrap();
            run(cpp
                .command(target.c_bridge_toolchain())
                .args(["-c", "-fPIC"])
                .arg(source)
                .arg("-o")
                .arg(&object));
            object
        });
        let source = directory.path().join("host.c");
        let host_object = directory.path().join("host.o");
        std::fs::write(&source, include_str!("shared/host.c")).unwrap();
        run(&mut target
            .c_bridge_toolchain()
            .object_compilation_command(&source, &host_object));
        let host = directory.path().join("host");
        let mut command = executable.command(directory.path(), &host).unwrap();
        command.arg(host_object);
        match &executable {
            ValidatedFinalLinkProfile::Darwin(profile) => {
                profile.linker_argument(&mut command, "-export_dynamic");
            }
            ValidatedFinalLinkProfile::Linux(_) => {
                command.args(["-Wl,--export-dynamic", "-ldl", "-lpthread"]);
            }
            ValidatedFinalLinkProfile::Windows(_) => panic!("Unix test profile"),
        }
        system_libraries(&executable, &mut command, directory.path());
        run(&mut command);
        let darwin = matches!(executable, ValidatedFinalLinkProfile::Darwin(_));
        if darwin {
            darwin_support::assert_lld(&host);
        }
        let library = directory.path().join(if darwin {
            "application , space.dylib"
        } else {
            "application , space.so"
        });
        let mut fingerprints = vec![executable.fingerprint().unwrap()];
        for explicit in [false, true] {
            let loader_name = "custom , space";
            let options = FinalLinkOptions {
                output_kind: FinalOutputKind::SharedLibrary,
                install_name: (explicit && darwin).then(|| loader_name.to_owned()),
                soname: (explicit && !darwin).then(|| loader_name.to_owned()),
                ..Default::default()
            };
            let profile = target.final_link_with_cxx(&options, cxx).unwrap();
            assert_eq!(profile.output_kind(), FinalOutputKind::SharedLibrary);
            if let ValidatedFinalLinkProfile::Linux(profile) = &profile {
                assert_eq!(profile.mode(), LinkMode::Dynamic);
            }
            let fingerprint = profile.fingerprint().unwrap();
            assert!(!fingerprints.contains(&fingerprint));
            fingerprints.push(fingerprint);
            let mut command = profile.command(directory.path(), &library).unwrap();
            command.arg(&object);
            command.args(&cpp_object);
            system_libraries(&profile, &mut command, directory.path());
            run(&mut command);
            let bytes = std::fs::read(&library).unwrap();
            let file = object::File::parse(bytes.as_slice()).unwrap();
            assert_eq!(file.kind(), object::ObjectKind::Dynamic);
            let exports = file.exports().unwrap();
            assert_eq!(exports.len(), 1);
            assert_eq!(
                exports[0].name(),
                if darwin {
                    b"_scoop_app_init".as_slice()
                } else {
                    b"scoop_app_init"
                }
            );
            let expected_name = if explicit {
                loader_name.to_owned()
            } else {
                format!(
                    "{}{}",
                    if darwin { "@rpath/" } else { "" },
                    library.file_name().unwrap().to_str().unwrap()
                )
            };
            if darwin {
                darwin_support::assert_lld(&library);
                let output = run(Command::new("/usr/bin/otool").arg("-D").arg(&library));
                assert_eq!(
                    String::from_utf8(output.stdout).unwrap().lines().nth(1),
                    Some(expected_name.as_str())
                );
            } else {
                let output = run(Command::new("readelf").arg("-dW").arg(&library));
                let dynamic = String::from_utf8(output.stdout).unwrap();
                assert!(
                    dynamic.lines().any(|line| line.contains("(SONAME)")
                        && line.ends_with(&format!("[{expected_name}]"))),
                    "{dynamic}"
                );
                assert!(!dynamic.contains("(TEXTREL)"), "{dynamic}");
                let elf =
                    object::read::elf::ElfFile64::<object::Endianness>::parse(bytes.as_slice())
                        .unwrap();
                let endian = elf.endian();
                let metadata = file.section_by_name(".data.rel.ro.scoop").unwrap();
                assert!(metadata.size() >= 8);
                assert!(
                    elf.elf_program_headers().iter().any(|header| {
                        header.p_type(endian) == object::elf::PT_GNU_RELRO
                            && metadata.address() >= header.p_vaddr(endian)
                            && metadata.address() + metadata.size()
                                <= header.p_vaddr(endian) + header.p_memsz(endian)
                    }),
                    "metadata must remain in the shared library's RELRO range"
                );
            }
            if can_execute(triple) {
                let output = run(Command::new(&host).arg(&library));
                assert!(
                    String::from_utf8_lossy(&output.stdout).contains("hidden callbacks passed")
                );
            } else {
                eprintln!("{triple}: linked and inspected; execution requires a matching host");
            }
        }
    }
}

fn can_execute(triple: &str) -> bool {
    if !cfg!(target_os = "macos") {
        return true;
    }
    if cfg!(target_arch = "aarch64") && triple.starts_with("aarch64") {
        return true;
    }
    triple.starts_with("x86_64")
        && Command::new("/usr/bin/arch")
            .args(["-x86_64", "/usr/bin/true"])
            .scoop_output()
            .is_ok_and(|output| output.status.success())
}
