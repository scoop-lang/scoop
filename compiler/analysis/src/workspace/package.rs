use std::{
    collections::{BTreeMap, BTreeSet},
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use scoop_process::CommandExt;
use scoop_protocol::{
    CompilationPurpose, DiagnosticOriginV1, HostPathCarrier,
    package::{
        ManifestOverlayV1, ManifestOverlaysV1, PACKAGE_PROTOCOL_VERSION, PackageDiagnosticV1,
        PackageLockedPurposesV1, PackageMemberV1, PackageMembersV1, PreparedResolutionV1,
        PreparedWorkspaceV1,
    },
};
use serde::de::DeserializeOwned;

use super::{LoadError, ProjectConfiguration};
use crate::AnalysisHost;

pub(super) struct PackageInputs<'a> {
    configuration: &'a ProjectConfiguration,
    overlays: ManifestOverlaysV1,
}

impl<'a> PackageInputs<'a> {
    pub fn new(
        configuration: &'a ProjectConfiguration,
        host: &AnalysisHost,
        overlays: &BTreeSet<PathBuf>,
    ) -> Result<Self, LoadError> {
        let manifests = overlays
            .iter()
            .filter(|path| path.file_name().is_some_and(|name| name == "Cone.toml"))
            .filter_map(|path| {
                host.file_source(path)
                    .and_then(|id| host.source(id))
                    .map(|source| (path, source))
            })
            .map(|(path, source)| {
                Ok(ManifestOverlayV1 {
                    path: HostPathCarrier::from_path(path).map_err(|error| {
                        LoadError::new(scoop_source::DiagnosticCode::ICE2501, path, error)
                    })?,
                    text: source.text.as_str().into(),
                })
            })
            .collect::<Result<_, LoadError>>()?;
        Ok(Self {
            configuration,
            overlays: ManifestOverlaysV1 {
                version: PACKAGE_PROTOCOL_VERSION,
                manifests,
            },
        })
    }

    pub fn members(&self, root: &Path) -> Result<PackageMembersV1, LoadError> {
        let result: PackageMembersV1 =
            self.invoke(root, self.command("members", root), Some(&self.overlays))?;
        result
            .validate()
            .map_err(|error| LoadError::new(scoop_source::DiagnosticCode::E2502, root, error))?;
        Ok(result)
    }

    pub fn locked_purposes(
        &self,
        lockfile: &Path,
    ) -> Result<BTreeMap<String, Vec<CompilationPurpose>>, LoadError> {
        let mut command = Command::new(&self.configuration.package_manager);
        command
            .arg("locked-purposes")
            .arg(lockfile)
            .arg("--target")
            .arg(self.target());
        let result: PackageLockedPurposesV1 = self.invoke(lockfile, command, None)?;
        result.validate().map_err(|error| {
            LoadError::new(scoop_source::DiagnosticCode::E2502, lockfile, error)
        })?;
        Ok(result.purposes)
    }

    pub fn load_member(
        &self,
        root: &Path,
        member: &PackageMemberV1,
        purpose: CompilationPurpose,
        initialize_lockfile: bool,
    ) -> Result<PreparedResolutionV1, LoadError> {
        let manifest = root.join("Cone.toml");
        if initialize_lockfile {
            self.invoke::<serde_json::Value>(
                &manifest,
                self.dependency_command("resolve", root, member, purpose),
                None,
            )?;
        }
        let mut command = self.dependency_command("metadata", root, member, purpose);
        command.arg("--offline");
        let mut result: PreparedWorkspaceV1 =
            self.invoke(&manifest, command, Some(&self.overlays))?;
        result.validate().map_err(|error| {
            LoadError::new(scoop_source::DiagnosticCode::E2502, &manifest, error)
        })?;
        let resolution = result
            .resolutions
            .pop()
            .expect("validated workspace has a resolution");
        if !result.resolutions.is_empty()
            || resolution.root != member.path
            || resolution.coordinate != member.coordinate
            || resolution.purpose != purpose
            || resolution.target != self.target()
        {
            return Err(LoadError::new(
                scoop_source::DiagnosticCode::E2502,
                &manifest,
                "package manager returned a different member, target or purpose",
            ));
        }
        Ok(resolution)
    }

    fn dependency_command(
        &self,
        action: &str,
        root: &Path,
        member: &PackageMemberV1,
        purpose: CompilationPurpose,
    ) -> Command {
        let coordinate = &member.coordinate;
        let mut command = self.command(action, root);
        command
            .arg("-p")
            .arg(format!("{}:{}", coordinate.group(), coordinate.name()))
            .arg("--purpose")
            .arg(match purpose {
                CompilationPurpose::Production => "production",
                CompilationPurpose::Test => "test",
            })
            .arg("--target")
            .arg(self.target())
            .arg("--sysroot")
            .arg(&self.configuration.sysroot);
        for path in &self.configuration.artifact_search_roots {
            command.arg("--cone-path").arg(path);
        }
        if let Some(path) = &self.configuration.package_cache {
            command.arg("--package-cache-dir").arg(path);
        }
        if let Some(path) = &self.configuration.lockfile {
            command.arg("--lockfile").arg(path);
        }
        command
    }

    fn target(&self) -> &str {
        self.configuration
            .target
            .target()
            .contract()
            .canonical_triple()
    }

    fn command(&self, action: &str, root: &Path) -> Command {
        let mut command = Command::new(&self.configuration.package_manager);
        command.arg(action).arg(root).arg("--json");
        command
    }

    fn invoke<T: DeserializeOwned>(
        &self,
        path: &Path,
        mut command: Command,
        overlays: Option<&ManifestOverlaysV1>,
    ) -> Result<T, LoadError> {
        let bytes = if let Some(overlays) = overlays.filter(|value| !value.manifests.is_empty()) {
            command.arg("--overlays-stdin");
            Some(serde_json::to_vec(overlays).map_err(|error| {
                LoadError::new(scoop_source::DiagnosticCode::ICE2501, path, error)
            })?)
        } else {
            None
        };
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .scoop_spawn()
            .map_err(|error| {
                LoadError::new(
                    scoop_source::DiagnosticCode::E2202,
                    path,
                    format!(
                        "cannot run package manager {}: {error}",
                        self.configuration.package_manager.display()
                    ),
                )
            })?;
        let written = if let Some(bytes) = bytes {
            child.stdin.take().expect("piped stdin").write_all(&bytes)
        } else {
            drop(child.stdin.take());
            Ok(())
        };
        let output = child
            .wait_with_output()
            .map_err(|error| LoadError::new(scoop_source::DiagnosticCode::E2204, path, error))?;
        if !output.status.success() {
            if let Ok(record) = serde_json::from_slice::<PackageDiagnosticV1>(&output.stderr) {
                record.validate().map_err(|error| {
                    LoadError::new(scoop_source::DiagnosticCode::E2502, path, error)
                })?;
                let diagnostic = record.diagnostic;
                let mut error = LoadError::new(
                    scoop_source::DiagnosticCode::E2502,
                    path,
                    diagnostic.message(),
                );
                error.code = diagnostic.code().to_owned();
                error.notes = diagnostic.notes().to_vec();
                if let DiagnosticOriginV1::HostPathSpan { path, span } = diagnostic.origin() {
                    error.path = path.to_path_buf().map_err(|error| {
                        LoadError::new(
                            scoop_source::DiagnosticCode::E2502,
                            &self.configuration.root,
                            error,
                        )
                    })?;
                    error = error.with_span(span.start() as usize..span.end() as usize);
                }
                return Err(error);
            }
            return Err(LoadError::new(
                scoop_source::DiagnosticCode::E2203,
                path,
                format!(
                    "package manager failed ({}): {}",
                    output.status,
                    String::from_utf8_lossy(&output.stderr).trim()
                ),
            ));
        }
        written
            .map_err(|error| LoadError::new(scoop_source::DiagnosticCode::E2204, path, error))?;
        serde_json::from_slice(&output.stdout).map_err(|error| {
            LoadError::new(
                scoop_source::DiagnosticCode::E2502,
                path,
                format!("invalid package manager response: {error}"),
            )
        })
    }
}
