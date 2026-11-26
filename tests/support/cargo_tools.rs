//! Locate prebuilt workspace binaries used by Cargo tests.
use std::path::{Path, PathBuf};

pub fn companion(tool: &str) -> PathBuf {
    let name = format!("{tool}{}", std::env::consts::EXE_SUFFIX);
    if let Some(compiler) = std::env::var_os("SCOOP_TEST_PAIRED_SCOOPC") {
        let compiler = PathBuf::from(compiler);
        return if tool == "scoopc" {
            compiler
        } else {
            compiler.with_file_name(name)
        };
    }

    // Cargo runs tests beneath either <profile>/deps or <profile>/build.
    let test = std::env::current_exe().expect("locate the Cargo test executable");
    test.ancestors()
        .find(|path| path.ends_with("deps") || path.ends_with("build"))
        .and_then(Path::parent)
        .expect("locate the Cargo profile directory")
        .join(name)
}
