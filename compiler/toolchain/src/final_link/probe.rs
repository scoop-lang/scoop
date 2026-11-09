use super::*;
use scoop_process::CommandExt;

pub(super) fn check(profile: &DarwinFinalLinkProfile) -> Result<Vec<PathBuf>, ToolchainError> {
    for symbol in profile.linker_system_requirements() {
        if !profile.data.system.exports().contains_key(*symbol) {
            return Err(error(format!(
                "SDK is missing linker support symbol {symbol}"
            )));
        }
    }
    let directory = tempfile::Builder::new()
        .prefix("scoop-link-probe-")
        .tempdir()
        .map_err(error)?;
    let source = directory.path().join("main.c");
    let object = directory.path().join("main.o");
    std::fs::write(
        &source,
        b"#include <stdio.h>\nint main(void) { return puts(\"scoop\") < 0; }\nint scoop_app_init(const void *host, void *app, void *error) { (void)host; (void)app; (void)error; return main(); }\n",
    )
    .map_err(error)?;
    let dependencies = directory.path().join("probe.d");
    let output = profile
        .startup
        .object_compilation_command(&source, &object)
        .args(["-v", "-MD", "-MT", "link-probe", "-MF"])
        .arg(&dependencies)
        .scoop_output()
        .map_err(error)?;
    if !output.status.success() {
        return Err(error(format!(
            "startup C probe failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    let headers = super::super::dependencies::read(&dependencies, &source, &output.stderr)?;
    let sdk = profile.startup.sdk_root().map_err(error)?;
    let stub = directory.path().join("libSystem.tbd");
    let system = &profile.data.system;
    std::fs::write(
        &stub,
        crate::write_link_stub(
            scoop_slib::MachOArchitecture::for_target(profile.id()).map_err(error)?,
            system.install_name(),
            system.current_version(),
            system.compatibility_version(),
            system.exports(),
        )?,
    )
    .map_err(error)?;
    let executable = directory.path().join("program");
    let output = profile
        .command(sdk, directory.path(), &executable)?
        .arg(&object)
        .arg(stub)
        .scoop_output()
        .map_err(error)?;
    if !output.status.success() {
        return Err(error(format!(
            "Mach-O LLD probe failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    Ok(headers)
}
