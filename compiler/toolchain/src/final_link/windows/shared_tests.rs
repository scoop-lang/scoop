use super::*;
use object::Object;

#[test]
fn windows_dlls_load_and_link_through_the_selected_import_library() {
    let directory = tempfile::Builder::new()
        .prefix("scoop DLL 链接 , ")
        .tempdir()
        .unwrap();
    for (index, profile) in tests::profiles(FinalOutputKind::SharedLibrary, directory.path())
        .into_iter()
        .enumerate()
    {
        let scratch = directory.path().join(index.to_string());
        std::fs::create_dir_all(&scratch).unwrap();
        let source = scratch.join("library.c");
        let object = scratch.join("library.obj");
        std::fs::write(&source, include_str!("../shared/library.c")).unwrap();
        let mut compile = profile.startup.object_compilation_command(&source, &object);
        compile
            .args(["-g", "-gcodeview", "-I"])
            .arg(profile.unwind_prefix().join("include"));
        if profile.cxx() {
            compile.arg("-DSCOOP_TEST_CPP");
        }
        probe::run(&mut compile).unwrap();
        let library = scratch.join("application , space.dll");
        let pdb = scratch.join("application , space.pdb");
        let mut command = profile.command(&scratch, &library).unwrap();
        command.arg(object);
        if let Some(cxx) = &profile.data.cxx {
            let source = scratch.join("library.cpp");
            let object = scratch.join("library-cpp.obj");
            std::fs::write(&source, probe::CPP_SOURCE).unwrap();
            let mut compile = cxx.command(&profile.startup);
            if cxx.kind().microsoft() {
                compile
                    .args(["/c", "/Z7"])
                    .arg(source)
                    .arg(format!("/Fo{}", object.display()));
            } else {
                compile
                    .args(["-c", "-g", "-gcodeview"])
                    .arg(source)
                    .arg("-o")
                    .arg(&object);
            }
            probe::run(&mut compile).unwrap();
            command.arg(object);
        }
        profile.append_system_libraries(&mut command);
        profile.configure_outputs(&mut command, Some(&pdb), false);
        probe::run(&mut command).unwrap();
        let bytes = std::fs::read(&library).unwrap();
        probe::check_image(
            &bytes,
            profile.startup.target(),
            FinalOutputKind::SharedLibrary,
        )
        .unwrap();
        let file = object::File::parse(bytes.as_slice()).unwrap();
        assert_eq!(
            file.pdb_info().unwrap().unwrap().path(),
            b"application , space.pdb"
        );
        assert!(
            std::fs::read(&pdb)
                .unwrap()
                .starts_with(b"Microsoft C/C++ MSF 7.00")
        );
        let import_library = profile.import_library_path(&library).unwrap();
        assert!(import_library.is_file());
        assert_eq!(
            import_library.file_name().unwrap(),
            if profile.microsoft() {
                "application , space.lib"
            } else {
                "application , space.dll.a"
            }
        );
        assert!(
            profile
                .command_with_subsystem(&scratch, &library, scoop_identity::WindowsSubsystem::Gui)
                .is_err()
        );

        let host_profile = WindowsFinalLinkProfile::resolve(
            profile.startup.clone(),
            &FinalLinkOptions {
                sysroot: Some(directory.path().to_owned()),
                unwind_prefix: Some(profile.unwind_prefix().to_owned()),
                linker: Some(profile.data.linker.clone()),
                ..Default::default()
            },
            None,
        )
        .unwrap();
        let source = scratch.join("host.c");
        let object = scratch.join("host.obj");
        std::fs::write(&source, include_str!("../shared/host.c")).unwrap();
        for import in [false, true] {
            let mut compile = profile.startup.object_compilation_command(&source, &object);
            if import {
                compile.arg("-DSCOOP_TEST_IMPORT");
            }
            probe::run(&mut compile).unwrap();
            let host = scratch.join(if import {
                "import-host.exe"
            } else {
                "load-host.exe"
            });
            let mut command = host_profile.command(&scratch, &host).unwrap();
            command.arg(&object);
            if import {
                command.arg(&import_library);
            }
            host_profile.append_system_libraries(&mut command);
            probe::run(&mut command).unwrap();
            let output = match probe::run(Command::new(&host).arg(&library)) {
                Ok(output) => output,
                Err(error) => {
                    let path = directory.keep();
                    panic!("{error}; DLL outputs retained at {}", path.display());
                }
            };
            assert!(String::from_utf8_lossy(&output.stdout).contains("hidden callbacks passed"));
        }
    }
}
