use std::path::{Path, PathBuf};
use std::process::Command;

use scoop_lir::DarwinPackedVersionV1;
use scoop_process::CommandExt;

use super::{NonZeroUsize, ResolvedTargetProfile, SwiftBuildRequest, ToolchainError, error};

pub(super) struct SwiftTools {
    swift: PathBuf,
    pub(super) root: PathBuf,
    pub(super) sdk: PathBuf,
    triple: String,
    pub(super) minimum_os: DarwinPackedVersionV1,
}

impl SwiftTools {
    pub(super) fn resolve(
        request: &SwiftBuildRequest,
        target: &ResolvedTargetProfile,
    ) -> Result<Self, ToolchainError> {
        let root = toolchain_root(request.options.toolchain.as_deref())?;
        // Swift dispatches by argv[0]; resolving its symlink selects swift-frontend.
        let swift = root.join("usr/bin/swift");
        let version = run(Command::new(&swift).arg("--version"))?;
        let version_text = version
            .split("Swift version ")
            .nth(1)
            .and_then(|text| text.split_whitespace().next())
            .ok_or_else(|| error(format!("cannot determine Swift version: {version}")))?;
        let version = crate::c_bridge::parse_darwin_version(version_text, "Swift version")?;
        if version.components() < (6, 3, 0) {
            return Err(error("Swift 6.3 or later is required for @c"));
        }
        let c = target.c_bridge_toolchain();
        let minimum_os = c
            .profile()
            .contract()
            .deployment()
            .map_err(error)?
            .minimum_os();
        let architecture = match target.canonical_triple() {
            "aarch64-apple-darwin" => "arm64",
            "x86_64-apple-darwin" => "x86_64",
            _ => return Err(error("Swift package production requires a macOS target")),
        };
        Ok(Self {
            swift,
            root,
            sdk: c.sdk_root().map_err(error)?.to_path_buf(),
            triple: format!("{architecture}-apple-macosx{minimum_os}"),
            minimum_os,
        })
    }

    fn command(&self, verb: &str, package: &Path, scratch: &Path) -> Command {
        let mut command = Command::new(&self.swift);
        command
            .arg(verb)
            .arg("--package-path")
            .arg(package)
            .arg("--scratch-path")
            .arg(scratch)
            .arg("--cache-path")
            .arg(
                scratch
                    .parent()
                    .expect("Swift work directory")
                    .join("downloads"),
            )
            .arg("--disable-experimental-prebuilts")
            .args(["--manifest-cache", "none"])
            .env("SWIFT_EXEC", self.root.join("usr/bin/swiftc"))
            .env("SWIFT_EXEC_MANIFEST", self.root.join("usr/bin/swiftc"));
        command
    }

    pub(super) fn build(
        &self,
        request: &SwiftBuildRequest,
        scratch: &Path,
        jobs: NonZeroUsize,
    ) -> Result<PathBuf, ToolchainError> {
        let command = || {
            let mut command = self.command("build", &request.package, scratch);
            command
                .args([
                    "--build-system",
                    "swiftbuild",
                    "--configuration",
                    request.configuration,
                    "--product",
                    &request.product,
                ])
                .arg("--triple")
                .arg(&self.triple)
                .arg("--sdk")
                .arg(&self.sdk)
                .arg("--toolchain")
                .arg(&self.root)
                .arg("--jobs")
                .arg(jobs.to_string())
                .env("MACOSX_DEPLOYMENT_TARGET", self.minimum_os.to_string());
            if request.options.locked || request.shared_source {
                command.arg("--force-resolved-versions");
            }
            command
        };
        run(&mut command())?;
        let bin = run(command().arg("--show-bin-path"))?;
        let bin = PathBuf::from(bin.trim());
        if !bin.is_absolute() {
            return Err(error(format!(
                "Swift returned a non-absolute products path: {}",
                bin.display()
            )));
        }
        Ok(bin)
    }
}

pub(crate) fn toolchain_root(selected: Option<&Path>) -> Result<PathBuf, ToolchainError> {
    let swift = match selected {
        Some(root) => root.join("usr/bin/swift"),
        None => {
            PathBuf::from(run(Command::new("/usr/bin/xcrun").args(["--find", "swift"]))?.trim())
        }
    };
    swift
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .ok_or_else(|| error(format!("invalid Swift toolchain path {}", swift.display())))?
        .canonicalize()
        .map_err(error)
}

pub(super) fn run(command: &mut Command) -> Result<String, ToolchainError> {
    let output = command.scoop_output().map_err(|cause| {
        error(format!(
            "cannot run {}: {cause}",
            command.get_program().to_string_lossy()
        ))
    })?;
    if !output.status.success() {
        return Err(error(format!(
            "{} failed ({}):\n{}{}",
            command.get_program().to_string_lossy(),
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    String::from_utf8(output.stdout).map_err(error)
}
