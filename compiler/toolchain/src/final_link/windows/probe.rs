use super::*;
use object::{Object, ObjectSection};
use scoop_process::CommandExt;
use std::collections::BTreeSet;

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
