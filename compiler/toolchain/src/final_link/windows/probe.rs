use super::*;
use object::{Object, ObjectSection};
use scoop_process::CommandExt;
use scoop_wire::sha256;

pub(super) fn check(
    profile: &WindowsFinalLinkProfile,
    observed: &mut BTreeSet<PathBuf>,
) -> Result<Vec<SystemInput>, ToolchainError> {
    let directory = tempfile::Builder::new()
        .prefix("scoop-pe-link-")
        .tempdir()
        .map_err(error)?;
    let source = directory.path().join("probe.c");
    let object = directory.path().join("probe.obj");
    let depfile = directory.path().join("probe.d");
    std::fs::write(&source, SOURCE).map_err(error)?;
    let mut compile = profile.startup.object_compilation_command(&source, &object);
    compile
        .args(["-v", "-MD", "-MT", "link-probe", "-MF"])
        .arg(&depfile)
        .arg("-I")
        .arg(&profile.data.unwind_include);
    let compiled = run(&mut compile)?;
    observed.extend(super::super::dependencies::read(
        &depfile,
        &source,
        &compiled.stderr,
    )?);
    let mut objects = vec![object];
    if let Some(cxx) = &profile.data.cxx {
        let source = directory.path().join("cpp.cpp");
        let object = directory.path().join("cpp.obj");
        std::fs::write(&source, CPP_SOURCE).map_err(error)?;
        let mut compile = cxx.command(&profile.startup);
        if cxx.kind().microsoft() {
            compile
                .args(["/c", "/Z7"])
                .arg(format!("/Fo{}", object.display()));
        } else {
            compile
                .args(["-c", "-g", "-gcodeview"])
                .arg("-o")
                .arg(&object);
        }
        crate::windows_native::input(&mut compile, cxx.kind(), &source);
        run(&mut compile)?;
        objects.push(object);
    }
    let output = directory.path().join(
        if profile.data.output.kind() == FinalOutputKind::SharedLibrary {
            "probe.dll"
        } else {
            "probe.exe"
        },
    );
    let mut command = profile.command(directory.path(), &output)?;
    for object in objects {
        crate::windows_native::input(
            &mut command,
            crate::windows_native::kind(profile.linker_driver()),
            &object,
        );
    }
    if profile.cxx() {
        profile.linker_arg(
            &mut command,
            if profile.microsoft() {
                "/include:scoop_cpp_probe"
            } else {
                "--undefined=scoop_cpp_probe"
            },
        );
    }
    profile.append_system_libraries(&mut command);
    profile.trace(&mut command);
    let result = run(&mut command)?;
    let mut inputs: BTreeSet<_> = profile
        .traced_inputs(&result, directory.path())
        .into_iter()
        .collect();
    inputs.insert(profile.data.unwind_prefix.join("lib/libunwind.a"));
    let bytes = std::fs::read(&output).map_err(error)?;
    check_image(&bytes, profile.startup.target(), profile.data.output.kind())?;
    inputs
        .into_iter()
        .map(|path| {
            Ok(SystemInput {
                digest: sha256(&std::fs::read(&path).map_err(error)?),
                path,
            })
        })
        .collect()
}

pub(super) fn check_image(
    bytes: &[u8],
    target: scoop_lir::LirTargetProfile,
    kind: FinalOutputKind,
) -> Result<(), ToolchainError> {
    let machine = crate::windows_machine::WindowsToolchainMachine::select(target).map_err(error)?;
    let file = object::File::parse(bytes).map_err(error)?;
    let expected = match kind {
        FinalOutputKind::Executable => object::ObjectKind::Executable,
        FinalOutputKind::SharedLibrary => object::ObjectKind::Dynamic,
    };
    if file.format() != object::BinaryFormat::Pe
        || file.architecture() != machine.object_architecture()
        || file.kind() != expected
        || !file.is_64()
    {
        return Err(error(format!(
            "Windows link probe did not produce the selected PE32+ {} output kind",
            machine.name()
        )));
    }
    if kind == FinalOutputKind::SharedLibrary {
        let exports = file.exports().map_err(error)?;
        if exports.len() != 1
            || exports[0].name() != super::super::output::NATIVE_APP_ENTRY.as_bytes()
        {
            return Err(error("DLL probe must export only scoop_app_init"));
        }
    }
    if file
        .section_by_name(".pdata")
        .is_none_or(|section| section.size() == 0)
    {
        return Err(error("Windows link probe has no SEH function table"));
    }
    let imports = file.imports().map_err(error)?;
    let libraries: BTreeSet<_> = imports
        .iter()
        .map(|import| String::from_utf8_lossy(import.library()).to_ascii_lowercase())
        .collect();
    let legacy = libraries.contains("msvcrt.dll");
    let ucrt = libraries
        .iter()
        .any(|name| name == "ucrtbase.dll" || name.starts_with("api-ms-win-crt-"));
    match crate::windows_c::native_environment(target.id())? {
        scoop_target_data::NativeEnvironment::LlvmMingwMsvcrt => {
            if !legacy || ucrt {
                return Err(error(format!(
                    "MSVCRT target linked incompatible CRT imports: {libraries:?}"
                )));
            }
        }
        scoop_target_data::NativeEnvironment::MicrosoftUcrt
        | scoop_target_data::NativeEnvironment::LlvmMingwUcrt => {
            if !ucrt || legacy {
                return Err(error(format!(
                    "UCRT target linked incompatible CRT imports: {libraries:?}"
                )));
            }
        }
        environment => {
            return Err(error(format!(
                "unsupported Windows native environment {environment:?}"
            )));
        }
    }
    Ok(())
}

pub(super) fn run(command: &mut Command) -> Result<std::process::Output, ToolchainError> {
    let output = command.scoop_output().map_err(error)?;
    if !output.status.success() {
        return Err(error(format!(
            "Windows native tool {command:?} failed ({}): {}{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    Ok(output)
}

pub(super) const SOURCE: &str = r#"
#include <stdio.h>
#include <stdint.h>
#include <unwind.h>
static _Unwind_Reason_Code trace(struct _Unwind_Context *context, void *data) {
    (void)context; (*(int *)data)++; return _URC_NO_REASON;
}
int wmain(int argc, uint16_t **argv) {
    int count = 0;
    _Unwind_Backtrace(trace, &count);
    return count < 2 || argc < 1 || argv[0][0] == 0 || puts("PE CRT and LLVM unwind passed") < 0;
}
int scoop_app_init(const void *host, void *app, void *error) {
    (void)host; (void)app; (void)error;
    uint16_t name[] = {'s', 0}; uint16_t *argv[] = {name, 0};
    return wmain(1, argv);
}
"#;

pub(super) const CPP_SOURCE: &str = include_str!("../shared/library.cpp");
