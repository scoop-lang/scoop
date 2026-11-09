use super::*;
use scoop_toolchain::LinuxFinalLinkProfile;

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

pub(super) fn link(
    profile: &LinuxFinalLinkProfile,
    inputs: &ProgramInputs<'_>,
    directory: &Path,
    archives: &[PathBuf],
    candidate: &Path,
    keep_symbols: bool,
) -> Result<(), LinkError> {
    let mut command = profile.command(directory, candidate).map_err(error)?;
    if !keep_symbols {
        command.arg("-Xlinker").arg("--strip-all");
    }

    append_archives(&mut command, directory, archives)?;
    command.arg("-Xlinker").arg(format!(
        "--defsym={}={}",
        inputs.symbol("scoop_td_String"),
        inputs.string_target
    ));
    shared::append(inputs, directory, &mut command)?;
    profile.append_system_libraries(&mut command);
    let result = command
        .scoop_output()
        .map_err(|err| error(format!("cannot start ELF linker: {err}")))?;
    if !result.status.success() {
        return Err(error(format!(
            "ELF linker failed: {}",
            String::from_utf8_lossy(&result.stderr)
        )));
    }
    Ok(())
}
