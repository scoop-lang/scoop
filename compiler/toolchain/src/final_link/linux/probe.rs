use super::*;
use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::process::Output;

pub(super) fn driver_program(mut command: Command, name: &str) -> Result<PathBuf, ToolchainError> {
    let path = command
        .get_envs()
        .find(|(key, _)| *key == OsStr::new("PATH"))
        .and_then(|(_, value)| value)
        .ok_or_else(|| error("Linux C invocation is missing PATH"))?
        .to_owned();
    let output = run(command.arg(format!("-print-prog-name={name}")))?;
    let program = String::from_utf8(output.stdout).map_err(error)?;
    crate::paths::find_program(OsStr::new(program.trim()), &path)
}

pub(super) fn system_inputs(
    profile: &LinuxFinalLinkProfile,
) -> Result<Vec<PathBuf>, ToolchainError> {
    // Ask the selected driver to resolve its libraries, including linker scripts.
    // No source, startup object, program object or published output is needed.
    let mut command = profile.data.runtime.command(&profile.startup);
    command.args(profile.linker_args()).args([
        "-nostartfiles",
        "-Wl,-t",
        "-Wl,-e,0",
        "-o",
        "/dev/null",
    ]);
    if profile.cxx() {
        // A library operand makes the driver run its normal C++ library plan.
        command.arg("-lc");
    } else {
        command.arg("-nodefaultlibs");
    }
    profile.append_system_libraries(&mut command);
    let output = run(&mut command)?;
    let trace = String::from_utf8(output.stdout).map_err(error)?;
    let paths = trace
        .lines()
        .map(|line| Path::new(line.trim()))
        .filter(|path| {
            path.is_absolute()
                && path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.ends_with(".a") || name.contains(".so"))
        })
        .map(Path::to_owned)
        .collect::<BTreeSet<_>>();
    if !paths.iter().any(|path| {
        path.file_name()
            .is_some_and(|name| name == "libc.so" || name == "libc.a")
    }) {
        return Err(error(
            "linker trace does not identify the selected libc development input",
        ));
    }
    Ok(paths.into_iter().collect())
}

pub(super) fn run(command: &mut Command) -> Result<Output, ToolchainError> {
    let output = command.scoop_output().map_err(error)?;
    if !output.status.success() {
        return Err(error(format!(
            "Linux native tool {command:?} failed ({}): {}{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    Ok(output)
}

#[cfg(all(test, target_os = "linux", target_arch = "x86_64"))]
pub(super) const SOURCE: &str = "#include <stdio.h>\n#include <unwind.h>\n\
static _Unwind_Reason_Code trace(struct _Unwind_Context *context, void *data) {\n\
    (void)context; (*(int *)data)++; return _URC_NO_REASON;\n}\n\
static __attribute__((noinline)) int count_frames(void) {\n\
    int frames = 0; _Unwind_Backtrace(trace, &frames); return frames;\n}\n\
int main(void) { return count_frames() < 2 ||\n\
    puts(\"LLVM unwinder and target libc passed\") < 0; }\n\
int scoop_app_init(const void *host, void *app, void *error) {\n\
    (void)host; (void)app; (void)error; return main(); }\n";
