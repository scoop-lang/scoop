use super::*;
use object::Object;
use object::read::pe::ImageNtHeaders;

fn tool(name: &str) -> PathBuf {
    std::env::var_os(name)
        .map(PathBuf::from)
        .unwrap_or_else(|| panic!("{name} is required for Windows link validation"))
}

pub(super) fn profiles(kind: FinalOutputKind, sysroot: &Path) -> Vec<WindowsFinalLinkProfile> {
    let clang = tool("SCOOP_TEST_CLANG");
    let msvcrt = tool("SCOOP_TEST_MINGW_MSVCRT");
    let ucrt = tool("SCOOP_TEST_MINGW_UCRT");
    let cl = find_msvc_tools::find_tool("x86_64-pc-windows-msvc", "cl.exe").unwrap();
    let link = cl.path().with_file_name("link.exe");
    let cases = [
        (
            scoop_target_data::WINDOWS_X86_64_MSVC_UCRT,
            None,
            "SCOOP_TEST_UNWIND_MSVC",
            None,
            None,
        ),
        (
            scoop_target_data::WINDOWS_X86_64_MSVC_UCRT,
            None,
            "SCOOP_TEST_UNWIND_MSVC",
            Some(cl.path()),
            None,
        ),
        (
            scoop_target_data::WINDOWS_X86_64_MSVC_UCRT,
            None,
            "SCOOP_TEST_UNWIND_MSVC",
            Some(Path::new("clang-cl")),
            Some(link),
        ),
        (
            scoop_target_data::WINDOWS_X86_64_MINGW_MSVCRT,
            msvcrt.parent().and_then(Path::parent),
            "SCOOP_TEST_UNWIND_MSVCRT",
            Some(msvcrt.as_path()),
            None,
        ),
        (
            scoop_target_data::WINDOWS_X86_64_MINGW_UCRT,
            ucrt.parent().and_then(Path::parent),
            "SCOOP_TEST_UNWIND_UCRT",
            Some(ucrt.as_path()),
            None,
        ),
    ];
    let mut profiles = Vec::new();
    for (target, prefix, unwind, cxx, linker) in cases {
        let prefix = prefix.map(|root| std::fs::canonicalize(root).unwrap());
        let startup =
            crate::resolve_windows_c_toolchain(target, &clang, prefix.as_deref()).unwrap();
        let cxx = cxx.map(|driver| {
            let driver = if target.id() == scoop_target_data::WINDOWS_X86_64_MSVC_UCRT_ID {
                driver.to_owned()
            } else {
                driver.with_file_name("clang++.exe")
            };
            ValidatedCxxToolchain::resolve_with(&startup, Some(&driver)).unwrap()
        });
        let options = FinalLinkOptions {
            output_kind: kind,
            sysroot: Some(sysroot.to_owned()),
            unwind_prefix: Some(tool(unwind)),
            linker,
            ..Default::default()
        };
        profiles.push(
            match crate::ValidatedFinalLinkProfile::from_tools(startup, &options, cxx).unwrap() {
                crate::ValidatedFinalLinkProfile::Windows(profile) => profile,
                profile => panic!("Windows target selected {profile:?}"),
            },
        );
    }
    profiles
}

#[test]
fn windows_final_link_profiles_link_real_crt_unwind_cpp_and_pdb() {
    let directory = tempfile::Builder::new()
        .prefix("scoop PE 链接 , ")
        .tempdir()
        .unwrap();
    for (index, profile) in profiles(FinalOutputKind::Executable, directory.path())
        .into_iter()
        .enumerate()
    {
        let crt_name = if profile.microsoft() {
            "ucrt.lib"
        } else {
            // Both prefixes use this filename; check_image verifies the DLLs.
            "libmsvcrt.a"
        };
        let scratch = directory.path().join(index.to_string());
        std::fs::create_dir_all(&scratch).unwrap();
        let source = scratch.join("入口.c");
        let object = scratch.join("入口.obj");
        let mut source_text =
            probe::SOURCE.replace("argc < 1", "argc != 2 || argv[1][0] != 0x503c");
        if profile.cxx() {
            source_text = source_text
                .replace("int wmain", "extern int scoop_cpp_probe(int);\nint wmain")
                .replace(
                    "return count < 2",
                    "return scoop_cpp_probe(0) != 10 || scoop_cpp_probe(1) != 10 || count < 2",
                );
        }
        std::fs::write(&source, source_text).unwrap();
        probe::run(
            profile
                .startup
                .object_compilation_command(&source, &object)
                .arg("-g")
                .arg("-gcodeview")
                .arg("-I")
                .arg(profile.unwind_prefix().join("include")),
        )
        .unwrap();
        let binary = scratch.join("程序");
        let pdb = scratch.join("程序.pdb");
        let mut command = profile.command(&scratch, &binary).unwrap();
        command.arg(&object);
        if let Some(cxx) = &profile.data.cxx {
            let source = scratch.join("C++ 入口.cpp");
            let object = scratch.join("C++ 入口.obj");
            std::fs::write(&source, probe::CPP_SOURCE).unwrap();
            let mut compile = cxx.command(&profile.startup);
            if cxx.kind().microsoft() {
                compile
                    .args(["/c", "/Z7"])
                    .arg(&source)
                    .arg(format!("/Fo{}", object.display()));
            } else {
                compile
                    .args(["-c", "-g", "-gcodeview"])
                    .arg(&source)
                    .arg("-o")
                    .arg(&object);
            }
            probe::run(&mut compile).unwrap();
            command.arg(object);
        }
        profile.append_system_libraries(&mut command);
        profile.configure_outputs(&mut command, Some(&pdb), false);
        profile.trace(&mut command);
        let linked = probe::run(&mut command).unwrap();
        let inputs = profile.traced_inputs(&linked, &scratch, &[]);
        assert!(
            inputs
                .iter()
                .any(|path| path.file_name().is_some_and(|name| name == crt_name)),
            "actual {crt_name} is missing from {inputs:?}",
        );
        let output = match probe::run(Command::new(&binary).arg("值")) {
            Ok(output) => output,
            Err(error) => {
                let path = directory.keep();
                panic!("{error}; link outputs retained at {}", path.display());
            }
        };
        assert!(String::from_utf8_lossy(&output.stdout).contains("PE CRT"));
        let bytes = std::fs::read(&binary).unwrap_or_else(|error| {
            panic!(
                "case {index}: missing exact output {}: {error}",
                binary.display()
            )
        });
        probe::check_image(
            &bytes,
            profile.startup.target(),
            FinalOutputKind::Executable,
        )
        .unwrap();
        let pe = object::read::pe::PeFile64::parse(bytes.as_slice()).unwrap();
        let header = pe.nt_headers().optional_header();
        assert_eq!(
            header.size_of_stack_reserve.get(object::LittleEndian),
            8 * 1024 * 1024
        );
        assert_eq!(header.size_of_stack_commit.get(object::LittleEndian), 4096);
        let file = object::File::parse(bytes.as_slice()).unwrap();
        let info = file.pdb_info().unwrap().unwrap();
        assert_eq!(std::str::from_utf8(info.path()).unwrap(), "程序.pdb");
        assert!(
            std::fs::read(&pdb)
                .unwrap()
                .starts_with(b"Microsoft C/C++ MSF 7.00")
        );
    }
}
