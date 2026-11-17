//! SwiftPM product production, with metadata reuse before tool discovery.

use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};

use scoop_lir::OptimizationMode;
use scoop_manifest::{SwiftLibrary, SwiftLibraryFormat, SwiftLinkage};
use scoop_protocol::{HostPathCarrier, TargetSelectionRequestV1};
use scoop_wire::Digest256;
use serde::{Deserialize, Serialize};

use crate::build_record::{self, Input, Record, values};
use crate::{ResolvedTargetProfile, ToolchainError};

mod inputs;
mod package;
mod plan;
mod product;
mod tool;
pub use product::SwiftProduct;
pub(crate) use tool::toolchain_root;

#[derive(Clone, Debug, Default)]
pub struct SwiftBuildOptions {
    pub toolchain: Option<PathBuf>,
    pub locked: bool,
    pub offline: bool,
}

/// Only text configuration and declared environment are captured here. Resolving
/// the target, executing manifests and reading sources belong to a cache miss.
#[derive(Clone, Debug)]
pub struct SwiftBuildRequest {
    package: PathBuf,
    product: String,
    linkage: SwiftLinkage,
    format: SwiftLibraryFormat,
    inputs: Vec<PathBuf>,
    target: TargetSelectionRequestV1,
    configuration: &'static str,
    options: SwiftBuildOptions,
    shared_source: bool,
    directory: PathBuf,
    key: Digest256,
    package_key: Digest256,
}

pub enum PreparedSwiftBuild {
    Complete(Box<SwiftProduct>),
    Pending(Box<PendingSwiftBuild>),
    WaitingForLock,
}

/// The package lock is acquired by the scheduler, so waiting consumes no worker.
pub struct PendingSwiftBuild {
    request: SwiftBuildRequest,
    _package_lock: File,
}

impl SwiftBuildRequest {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        cone_root: &Path,
        library: &SwiftLibrary,
        target: TargetSelectionRequestV1,
        optimization: OptimizationMode,
        options: SwiftBuildOptions,
        shared_source: bool,
        cache_root: &Path,
    ) -> Result<Self, ToolchainError> {
        if !cfg!(target_os = "macos")
            || !matches!(
                target.canonical_triple(),
                "aarch64-apple-darwin" | "x86_64-apple-darwin"
            )
        {
            return Err(error(
                "Swift package production requires a macOS host and macOS target",
            ));
        }
        let package = cone_root.join(library.package().as_str());
        let package = package.canonicalize().map_err(error)?;
        let inputs = library
            .inputs()
            .iter()
            .map(|p| cone_root.join(p.as_str()))
            .collect::<Vec<_>>();
        let configuration = match optimization {
            OptimizationMode::Debug => "debug",
            OptimizationMode::Release => "release",
        };
        let environment = library
            .env()
            .iter()
            .map(String::as_str)
            .chain([
                "DEVELOPER_DIR",
                "TOOLCHAINS",
                "SDKROOT",
                "PATH",
                "SWIFT_EXEC",
                "SWIFT_EXEC_MANIFEST",
                "SWIFTPM_CUSTOM_LIBS_DIR",
                "PKG_CONFIG_PATH",
            ])
            .map(|key| {
                (
                    key,
                    std::env::var_os(key).map(|value| value.as_encoded_bytes().to_vec()),
                )
            })
            .collect::<BTreeMap<_, _>>();
        let path = |path: &Path| HostPathCarrier::from_path(path).map_err(error);
        let package_key = build_record::fingerprint(&path(&package)?)?;
        // Layout is deliberately absent: a framework can reuse the same product.
        let key = build_record::fingerprint(&(
            4u32,
            path(&package)?,
            library.product().as_str(),
            library.linkage(),
            (
                target.canonical_triple(),
                target.compiler(),
                target.native_sysroot(),
                target.cxx_compiler(),
                target.sysroot(),
                target.resolved_toolchain(),
            ),
            configuration,
            options.toolchain.as_deref().map(path).transpose()?,
            inputs
                .iter()
                .map(|p| path(p))
                .collect::<Result<Vec<_>, _>>()?,
            environment,
        ))?;
        Ok(Self {
            package,
            product: library.product().as_str().to_owned(),
            linkage: library.linkage(),
            format: library.format().clone(),
            inputs,
            target,
            configuration,
            options,
            shared_source,
            directory: cache_root.join("swift"),
            key,
            package_key,
        })
    }

    pub fn product(&self) -> &str {
        &self.product
    }
    pub fn key(&self) -> Digest256 {
        self.key
    }

    pub fn prepare(&self, wait: bool) -> Result<PreparedSwiftBuild, ToolchainError> {
        if let Some(product) = self.lookup()? {
            return Ok(PreparedSwiftBuild::Complete(Box::new(product)));
        }
        // This lock must span explicit cache roots, targets and toolchain
        // versions: all of them can update the same Package.resolved.
        let locks = crate::paths::default_cache()
            .ok_or_else(|| error("no platform cache directory for Swift package locks"))?
            .join("swift-package-locks");
        std::fs::create_dir_all(&locks).map_err(error)?;
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(locks.join(format!("{}.lock", self.package_key)))
            .map_err(error)?;
        if wait {
            fs4::FileExt::lock(&lock).map_err(error)?;
        } else {
            match fs4::FileExt::try_lock(&lock) {
                Ok(()) => {}
                Err(fs4::TryLockError::WouldBlock) => {
                    return Ok(PreparedSwiftBuild::WaitingForLock);
                }
                Err(fs4::TryLockError::Error(cause)) => return Err(error(cause)),
            }
        }
        if let Some(product) = self.lookup()? {
            return Ok(PreparedSwiftBuild::Complete(Box::new(product)));
        }
        Ok(PreparedSwiftBuild::Pending(Box::new(PendingSwiftBuild {
            request: self.clone(),
            _package_lock: lock,
        })))
    }

    fn lookup(&self) -> Result<Option<SwiftProduct>, ToolchainError> {
        let path = self
            .directory
            .join("records")
            .join(format!("{}.json", self.key));
        let Some(record) = build_record::lookup::<Produced>(&path, Produced::available)? else {
            return Ok(None);
        };
        product::lookup_layout(self, &record.result)
    }
}

impl PendingSwiftBuild {
    pub fn build(
        self,
        target: &ResolvedTargetProfile,
        jobs: NonZeroUsize,
    ) -> Result<SwiftProduct, ToolchainError> {
        let request = &self.request;
        if target.canonical_triple() != request.target.canonical_triple() {
            return Err(error(
                "Swift build target differs from the prepared request",
            ));
        }
        let record = build_record::cached(
            &request.directory.join("records"),
            request.key,
            Produced::available,
            || {
                let tools = tool::SwiftTools::resolve(request, target)?;
                // Supplementary inputs need not be known to SwiftPM's own
                // incremental engine. A changed declaration input selects new
                // work state so macros and generators actually consume it.
                let mut supplemental = Vec::new();
                for path in &request.inputs {
                    supplemental.push(Input::read(path)?);
                    let selection = inputs::Selection::read(path.clone(), inputs::Rule::All)?;
                    supplemental.extend(
                        selection
                            .files()
                            .iter()
                            .map(|p| Input::read(p))
                            .collect::<Result<Vec<_>, _>>()?,
                    );
                }
                let work_key = build_record::fingerprint(&(request.key.as_array(), supplemental))?;
                let scratch = request.directory.join("work").join(work_key.to_string());
                let mut packages = package::Packages::prepare(request, &tools, &scratch)?;
                let bin = tools.build(request, &scratch, jobs)?;
                let plan =
                    plan::BuildPlan::read(&scratch, &bin, &request.product, request.linkage)?;
                packages.discover(request, &tools, &scratch)?;
                let inputs = packages.inputs(request, &tools, &scratch, &plan)?;
                let result = product::publish(
                    request,
                    &bin,
                    &plan,
                    inputs.selections,
                    inputs.settings,
                    inputs.binaries,
                    packages.has_framework_module(request),
                )?;
                Ok(Record {
                    inputs: inputs.inputs,
                    result,
                })
            },
        )?;
        product::layout(request, &record.result)
    }
}

#[derive(Serialize, Deserialize)]
struct Produced {
    product: SwiftProduct,
    selections: Vec<inputs::Selection>,
    #[serde(with = "values::paths")]
    required: Vec<PathBuf>,
    #[serde(with = "values::paths")]
    modules: Vec<PathBuf>,
}

impl Produced {
    fn available(&self) -> Result<bool, ToolchainError> {
        for selection in &self.selections {
            if !selection.is_current()? {
                return Ok(false);
            }
        }
        outputs_exist(&self.required)
    }
}

fn outputs_exist(paths: &[PathBuf]) -> Result<bool, ToolchainError> {
    for path in paths {
        if !path.try_exists().map_err(error)? {
            return Ok(false);
        }
    }
    Ok(true)
}

fn error(value: impl std::fmt::Display) -> ToolchainError {
    ToolchainError::new(
        scoop_source::DiagnosticCode::E2201,
        format!("Swift package: {value}"),
    )
}

#[cfg(test)]
mod tests;
