use super::*;
use crate::NativeLibraryDependency;

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Container {
    available_libraries: Vec<Slice>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Slice {
    library_identifier: String,
    library_path: PathBuf,
    supported_platform: String,
    supported_platform_variant: Option<String>,
    supported_architectures: Vec<String>,
}

impl Packages {
    pub(super) fn binary_inputs(
        &self,
        request: &SwiftBuildRequest,
        descriptions: &BTreeMap<PathBuf, Description>,
        plan: &BuildPlan,
        files: &mut BTreeSet<PathBuf>,
        selections: &mut Vec<Selection>,
    ) -> Result<Vec<NativeLibraryDependency>, ToolchainError> {
        let consumers = self.consumer_binaries(descriptions, plan)?;
        let architecture = match request.target.canonical_triple() {
            "aarch64-apple-darwin" => "arm64",
            "x86_64-apple-darwin" => "x86_64",
            _ => return Err(error("XCFramework input requires a macOS target")),
        };
        let mut binaries = Vec::new();
        for (source, outputs) in &plan.binary_artifacts {
            let manifest = source.join("Info.plist");
            files.insert(manifest.clone());
            let container: Container = plist::from_file(&manifest)
                .map_err(|cause| error(format!("{}: {cause}", manifest.display())))?;
            let candidates = container
                .available_libraries
                .iter()
                .filter(|slice| {
                    slice.supported_platform == "macos"
                        && slice.supported_platform_variant.is_none()
                        && slice
                            .supported_architectures
                            .iter()
                            .any(|arch| arch == architecture)
                })
                .collect::<Vec<_>>();
            let [slice] = candidates.as_slice() else {
                return Err(error(format!(
                    "{} requires one macOS {architecture} slice",
                    source.display()
                )));
            };
            add_selection(
                source.join(&slice.library_identifier),
                Rule::All,
                files,
                selections,
            )?;
            if !consumers.contains(source) {
                continue;
            }
            let basename = slice
                .library_path
                .file_name()
                .ok_or_else(|| error("XCFramework library has no basename"))?;
            let candidates = outputs
                .iter()
                .filter_map(|output| {
                    output
                        .ancestors()
                        .find(|path| path.file_name() == Some(basename))
                        .map(Path::to_owned)
                })
                .collect::<BTreeSet<_>>();
            let library = match candidates.into_iter().collect::<Vec<_>>().as_slice() {
                [path] => path.clone(),
                _ => {
                    return Err(error(format!(
                        "Swift ProcessXCFramework has no unique output for {}",
                        slice.library_path.display()
                    )));
                }
            };
            let framework = library
                .extension()
                .is_some_and(|extension| extension == "framework");
            let name = library
                .file_stem()
                .and_then(|name| name.to_str())
                .ok_or_else(|| error("invalid XCFramework library name"))?;
            let name = if framework {
                name
            } else {
                name.strip_prefix("lib")
                    .ok_or_else(|| error("XCFramework library filename must start with lib"))?
            }
            .to_owned();
            let binary = if framework {
                library.join(&name).canonicalize().map_err(error)?
            } else {
                library.clone()
            };
            let linkage = binary_linkage(&binary, architecture)?;
            if request.linkage == SwiftLinkage::Dynamic && linkage == SwiftLinkage::Static {
                continue;
            }
            binaries.push(NativeLibraryDependency {
                name,
                linkage,
                framework,
                library,
                binary,
            });
        }
        Ok(binaries)
    }

    fn consumer_binaries(
        &self,
        descriptions: &BTreeMap<PathBuf, Description>,
        plan: &BuildPlan,
    ) -> Result<BTreeSet<PathBuf>, ToolchainError> {
        let mut selected = BTreeSet::new();
        for (path, description) in descriptions {
            let mut add = |owner: &Path, name: &str| -> Result<(), ToolchainError> {
                if let Some(target) = descriptions[owner]
                    .targets
                    .iter()
                    .find(|target| target.name == name && target.kind == "binary")
                {
                    selected.insert(owner.join(&target.path).canonicalize().map_err(error)?);
                }
                Ok(())
            };
            for target in description
                .targets
                .iter()
                .filter(|target| target.linked(path, plan))
            {
                for dependency in &target.target_dependencies {
                    add(path, dependency)?;
                }
                let raw = self.manifests[path]
                    .targets
                    .iter()
                    .find(|raw| raw["name"].as_str() == Some(&target.name))
                    .expect("described target has a manifest");
                for name in &target.product_dependencies {
                    let package = raw["dependencies"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .find(|dependency| dependency["product"][0].as_str() == Some(name))
                        .and_then(|dependency| dependency["product"][1].as_str());
                    for (identity, dependency) in &self.dependencies[path] {
                        let manifest = &self.manifests[dependency];
                        if package.is_some_and(|name| {
                            !self.matches_package(path, identity, manifest, name)
                        }) {
                            continue;
                        }
                        if let Some(product) = manifest
                            .products
                            .iter()
                            .find(|product| product["name"].as_str() == Some(name))
                        {
                            for target in product["targets"]
                                .as_array()
                                .into_iter()
                                .flatten()
                                .filter_map(Value::as_str)
                            {
                                add(dependency, target)?;
                            }
                        }
                    }
                }
            }
        }
        Ok(selected)
    }
}

fn binary_linkage(path: &Path, architecture: &str) -> Result<SwiftLinkage, ToolchainError> {
    use object::read::macho::{FatArch, MachOFatFile32, MachOFatFile64};
    let bytes = std::fs::read(path).map_err(error)?;
    fn slice<'a, F: FatArch>(
        bytes: &'a [u8],
        arches: &[F],
        architecture: &str,
    ) -> Result<&'a [u8], ToolchainError> {
        let cpu = if architecture == "arm64" {
            object::macho::CPU_TYPE_ARM64
        } else {
            object::macho::CPU_TYPE_X86_64
        };
        arches
            .iter()
            .find(|arch| arch.cputype() == cpu)
            .ok_or_else(|| error("binary input has no selected architecture"))?
            .data(bytes)
            .map_err(error)
    }
    let selected = match object::FileKind::parse(bytes.as_slice()).map_err(error)? {
        object::FileKind::MachOFat32 => slice(
            &bytes,
            MachOFatFile32::parse(bytes.as_slice())
                .map_err(error)?
                .arches(),
            architecture,
        )?,
        object::FileKind::MachOFat64 => slice(
            &bytes,
            MachOFatFile64::parse(bytes.as_slice())
                .map_err(error)?
                .arches(),
            architecture,
        )?,
        _ => bytes.as_slice(),
    };
    match object::FileKind::parse(selected).map_err(error)? {
        object::FileKind::Archive => Ok(SwiftLinkage::Static),
        object::FileKind::MachO64 => Ok(SwiftLinkage::Dynamic),
        _ => Err(error(format!("{} is not a native library", path.display()))),
    }
}
