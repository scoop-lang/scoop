use super::*;
use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::process::Output;

pub(super) fn linker(
    startup: &ValidatedCBridgeToolchainInvocation,
    runtime: &RuntimeLibraries,
) -> Result<PathBuf, ToolchainError> {
    let command = runtime.command(startup);
    if startup.target().architecture() != scoop_lir::TargetArchitecture::Aarch64 {
        return driver_program(command, "ld");
    }
    let packaged = crate::linux_c::bundled_llvm_tool("ld.lld")?;
    let sdk = std::env::var_os("LLVM_SYS_221_PREFIX")
        .map(|prefix| PathBuf::from(prefix).join("bin/ld.lld"));
    if let Some(path) = std::iter::once(packaged)
        .chain(sdk)
        .find(|path| path.is_file())
    {
        return std::path::absolute(path)
            .map_err(|cause| error(scoop_source::DiagnosticCode::E2201, cause));
    }
    driver_program(command, "ld.lld")
}

fn driver_program(mut command: Command, name: &str) -> Result<PathBuf, ToolchainError> {
    let path = command
        .get_envs()
        .find(|(key, _)| *key == OsStr::new("PATH"))
        .and_then(|(_, value)| value)
        .ok_or_else(|| {
            error(
                scoop_source::DiagnosticCode::ICE2201,
                "Linux C invocation is missing PATH",
            )
        })?
        .to_owned();
    let output = run(command.arg(format!("-print-prog-name={name}")))?;
    let program = String::from_utf8(output.stdout)
        .map_err(|cause| error(scoop_source::DiagnosticCode::E2303, cause))?;
    crate::paths::find_program(OsStr::new(program.trim()), &path)
}

pub(super) fn system_inputs(
    profile: &LinuxFinalLinkProfile,
) -> Result<Vec<PathBuf>, ToolchainError> {
    // Ask the selected driver to resolve its libraries, including linker scripts.
    // No source, startup object, program object or published output is needed.
    let scratch =
        tempfile::tempdir().map_err(|cause| error(scoop_source::DiagnosticCode::E2201, cause))?;
    let depfile = scratch.path().join("link-probe.d");
    let output = scratch.path().join("link-probe");
    let mut command = profile.driver_command();
    command
        .args(profile.linker_args())
        .args(["-nostartfiles", "-Xlinker", "--dependency-file", "-Xlinker"])
        .arg(&depfile)
        .args(["-Wl,-e,0", "-o"])
        .arg(&output);
    if profile.cxx() {
        // A library operand makes the driver run its normal C++ library plan.
        command.arg("-lc");
    } else {
        command.arg("-nodefaultlibs");
    }
    profile.append_system_libraries(&mut command);
    run(&mut command)?;
    linker_libraries(&depfile, &output)
}

pub(super) fn linker_libraries(
    depfile: &Path,
    output: &Path,
) -> Result<Vec<PathBuf>, ToolchainError> {
    // The depfile includes linker scripts and archives even if no member was extracted.
    let dependencies = std::fs::read_to_string(depfile)
        .map_err(|cause| error(scoop_source::DiagnosticCode::E2303, cause))?;
    let first_rule = dependencies
        .split_once("\n\n")
        .map_or(dependencies.as_str(), |(rule, _)| rule);
    let output = output.to_string_lossy();
    // GNU ld writes one literal prerequisite per line with two spaces of indentation.
    // LLD writes make-escaped prerequisites, including escaped spaces and dollars.
    let inputs = if let Some(raw) = first_rule
        .strip_prefix(output.as_ref())
        .and_then(|rule| rule.strip_prefix(": \\\n  "))
    {
        raw.split(" \\\n  ").map(PathBuf::from).collect()
    } else {
        scoop_process::parse_make_dependencies(first_rule, &output).ok_or_else(|| {
            error(
                scoop_source::DiagnosticCode::E2303,
                "invalid Linux linker dependency file",
            )
        })?
    };
    let paths = inputs
        .into_iter()
        .filter(|path| {
            path.is_absolute()
                && path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.ends_with(".a") || name.contains(".so"))
        })
        .collect::<BTreeSet<_>>();
    if !paths.iter().any(|path| {
        path.file_name()
            .is_some_and(|name| name == "libc.so" || name == "libc.a")
    }) {
        return Err(error(
            scoop_source::DiagnosticCode::E2303,
            "linker dependencies do not identify the selected libc development input",
        ));
    }
    Ok(paths.into_iter().collect())
}

pub(super) fn run(command: &mut Command) -> Result<Output, ToolchainError> {
    let output = command
        .scoop_output()
        .map_err(|cause| error(scoop_source::DiagnosticCode::E2202, cause))?;
    if !output.status.success() {
        return Err(error(
            scoop_source::DiagnosticCode::E2203,
            format!(
                "Linux native tool {command:?} failed ({}): {}{}",
                output.status,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ),
        ));
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
