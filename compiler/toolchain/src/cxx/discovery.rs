use super::*;

pub(super) fn paired_driver(
    c: &ValidatedCBridgeToolchainInvocation,
) -> Result<PathBuf, ToolchainError> {
    let path = c.compiler_driver();
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    let paired = match c.native_environment() {
        scoop_target_data::NativeEnvironment::Darwin => "clang++".to_owned(),
        scoop_target_data::NativeEnvironment::LinuxGnu
            if c.compiler_family() == NativeCompilerFamily::Clang =>
        {
            match name.rsplit_once("clang") {
                Some((prefix, version)) => format!("{prefix}clang++{version}"),
                None if name == "cc" => "c++".to_owned(),
                None => {
                    return Err(error(format!(
                        "cannot locate a paired Clang C++ driver for {}",
                        path.display()
                    )));
                }
            }
        }
        scoop_target_data::NativeEnvironment::LinuxGnu => match name.rsplit_once("gcc") {
            Some((prefix, version)) => format!("{prefix}g++{version}"),
            None if name == "cc" => "c++".to_owned(),
            None => {
                return Err(error(format!(
                    "cannot locate a paired C++ driver for {}",
                    path.display()
                )));
            }
        },
        scoop_target_data::NativeEnvironment::LinuxMusl => {
            return Err(error("C++ is not supported for Linux musl"));
        }
        _ => return Err(error("unsupported target for C++ driver selection")),
    };
    // Preserve clang++'s spelling: its argv[0] selects C++ link defaults.
    let driver = path.with_file_name(paired);
    if !driver.is_file() {
        return Err(error(format!(
            "missing paired C++ driver {}",
            driver.display()
        )));
    }
    Ok(driver)
}
