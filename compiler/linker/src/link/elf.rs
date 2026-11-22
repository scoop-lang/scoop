//! Inputs and flags for an already selected GNU-compatible ELF linker command.

use super::*;

mod shared;

#[cfg(all(test, target_os = "linux"))]
mod tests;

fn append_archives(
    command: &mut std::process::Command,
    directory: &Path,
    archives: &[PathBuf],
) -> Result<(), LinkError> {
    response::write_paths(directory, archives)?;
    command
        .current_dir(directory)
        .args(["-Xlinker", "--whole-archive"])
        .arg(format!("-Wl,@{}", response::FILE_NAME))
        .args(["-Xlinker", "--no-whole-archive"]);
    Ok(())
}

pub(super) fn append_inputs(
    command: &mut std::process::Command,
    inputs: &ProgramInputs<'_>,
    directory: &Path,
    archives: &[PathBuf],
    keep_symbols: bool,
    dead_strip: bool,
) -> Result<(), LinkError> {
    if !keep_symbols {
        command.arg("-Xlinker").arg("--strip-all");
    }
    if dead_strip {
        command.arg("-Xlinker").arg("--gc-sections");
    }

    append_archives(command, directory, archives)?;
    command.arg("-Xlinker").arg(format!(
        "--defsym={}={}",
        inputs.symbol("scoop_td_String"),
        inputs.string_target
    ));
    shared::append(inputs, directory, command)?;
    Ok(())
}

pub(super) fn run(command: &mut std::process::Command) -> Result<(), LinkError> {
    let result = command.scoop_output().map_err(|err| {
        error(
            scoop_source::DiagnosticCode::E2202,
            format!("cannot start ELF linker: {err}"),
        )
    })?;
    if !result.status.success() {
        return Err(error(
            scoop_source::DiagnosticCode::E2304,
            format!(
                "ELF linker failed: {}",
                String::from_utf8_lossy(&result.stderr)
            ),
        ));
    }
    Ok(())
}
