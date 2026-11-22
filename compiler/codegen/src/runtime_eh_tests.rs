use std::path::{Path, PathBuf};
use std::process::Command;

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("codegen crate is nested below the workspace root")
        .to_path_buf()
}

#[test]
fn closed_lsda_decoder_accepts_only_the_qualified_profile() {
    let workspace = workspace_root();
    let directory = tempfile::tempdir().unwrap();
    let binary = directory.path().join("eh-decoder");
    for target in [
        scoop_target_data::DARWIN_AARCH64,
        scoop_target_data::WINDOWS_X86_64_MSVC_UCRT,
    ] {
        let profile = scoop_toolchain::ValidatedRuntimeBuildProfile::for_target(target).unwrap();
        let lsda_flags = profile
            .runtime_c_flags()
            .into_iter()
            .filter(|flag| flag.starts_with("-DSCOOP_EH_"));
        let compile = Command::new("cc")
            .args(["-std=c11", "-Wall", "-Wextra", "-Werror"])
            .args(lsda_flags)
            .arg("-I")
            .arg(workspace.join("runtime/src"))
            .arg(workspace.join("runtime/src/eh_personality.c"))
            .arg(workspace.join("runtime/src/eh/lsda.c"))
            .arg(workspace.join("runtime/tests/eh_decoder_test.c"))
            .arg("-o")
            .arg(&binary)
            .output()
            .expect("compile the bounded LSDA decoder test");
        assert!(
            compile.status.success(),
            "bounded LSDA decoder test for {} must compile cleanly:\n{}",
            profile.canonical_triple(),
            String::from_utf8_lossy(&compile.stderr)
        );

        let output = Command::new(&binary)
            .output()
            .expect("run the bounded LSDA decoder test");
        assert!(
            output.status.success(),
            "bounded LSDA decoder test for {} failed:\nstdout:\n{}\nstderr:\n{}",
            profile.canonical_triple(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output.stdout, b"EH decoder tests passed\n");
    }
}
