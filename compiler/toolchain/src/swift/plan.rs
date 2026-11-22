//! Locate actual build outputs; SwiftPM owns its package and target graph.
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::Deserialize;

use super::{SwiftLinkage, ToolchainError, error};

#[derive(Deserialize)]
struct Manifest {
    targets: BTreeMap<String, Vec<String>>,
    commands: BTreeMap<String, BuildCommand>,
}

#[derive(Deserialize)]
struct BuildCommand {
    #[serde(default)]
    inputs: Vec<String>,
    #[serde(default)]
    outputs: Vec<String>,
    #[serde(default, rename = "expectedOutputs")]
    expected_outputs: Vec<String>,
}

pub(super) struct BuildPlan {
    pub(super) outputs: BTreeSet<PathBuf>,
}

impl BuildPlan {
    pub(super) fn read(
        scratch: &Path,
        bin: &Path,
        product: &str,
        linkage: SwiftLinkage,
    ) -> Result<Self, ToolchainError> {
        let data = scratch.join("out/Intermediates.noindex/XCBuildData");
        // Swift Build records access by touching the description package. Use
        // that access order, not manifest mtime (a reused plan is not rewritten).
        let mut descriptions = std::fs::read_dir(&data)
            .map_err(|cause| {
                error(format!(
                    "Swift Build descriptions at {}: {cause}",
                    data.display()
                ))
            })?
            .filter_map(|entry| match entry {
                Ok(entry)
                    if entry
                        .path()
                        .extension()
                        .is_some_and(|extension| extension == "xcbuilddata") =>
                {
                    Some(Ok(entry.path()))
                }
                Ok(_) => None,
                Err(cause) => Some(Err(cause)),
            })
            .map(|entry| {
                let path = entry?;
                Ok((path.metadata()?.modified()?, path))
            })
            .collect::<Result<Vec<_>, std::io::Error>>()
            .map_err(error)?;
        descriptions.sort();
        let (_, description) = descriptions
            .pop()
            .ok_or_else(|| error("Swift Build did not produce a build description"))?;
        let path = description.join("manifest.json");
        let manifest: Manifest =
            serde_json::from_reader(std::fs::File::open(&path).map_err(error)?).map_err(error)?;
        let producers = manifest
            .commands
            .iter()
            .flat_map(|(id, command)| {
                command
                    .outputs
                    .iter()
                    .map(move |output| (output.as_str(), id.as_str()))
            })
            .collect::<BTreeMap<_, _>>();
        let mut queue = manifest
            .targets
            .values()
            .flatten()
            .map(String::as_str)
            .collect::<Vec<_>>();
        let mut visited = BTreeSet::new();
        let mut commands = Vec::new();
        while let Some(input) = queue.pop() {
            if let Some(id) = producers.get(input) {
                if !visited.insert(*id) {
                    continue;
                }
                let command = &manifest.commands[*id];
                queue.extend(command.inputs.iter().map(String::as_str));
                commands.push(command);
            }
        }
        let outputs = commands
            .iter()
            .flat_map(|command| command.outputs.iter().chain(&command.expected_outputs))
            .filter_map(|output| file_path(output))
            .collect::<BTreeSet<_>>();
        let main = bin.join(format!(
            "lib{product}.{}",
            match linkage {
                SwiftLinkage::Static => "a",
                SwiftLinkage::Dynamic => "dylib",
            }
        ));
        let main = main
            .canonicalize()
            .map_err(|cause| error(format!("missing Swift product {}: {cause}", main.display())))?;
        if !outputs.contains(&main) {
            return Err(error(format!(
                "selected Swift Build description does not produce {}",
                main.display()
            )));
        }
        Ok(Self { outputs })
    }

    pub(super) fn binaries(
        &self,
        request: &super::SwiftBuildRequest,
        bin: &Path,
    ) -> Result<Vec<crate::NativeLibraryDependency>, ToolchainError> {
        let bin = bin.canonicalize().map_err(error)?;
        let main = bin.join(format!(
            "lib{}.{}",
            request.product,
            if request.linkage == SwiftLinkage::Static {
                "a"
            } else {
                "dylib"
            }
        ));
        let mut libraries = BTreeSet::new();
        for output in &self.outputs {
            let Ok(relative) = output.strip_prefix(&bin) else {
                continue;
            };
            let Some(first) = relative.components().next() else {
                continue;
            };
            let library = bin.join(first);
            if library == main {
                continue;
            }
            if library
                .extension()
                .is_some_and(|ext| ext == "framework" || ext == "a" || ext == "dylib")
            {
                libraries.insert(library);
            }
        }
        let target = scoop_target_data::named_target(request.target.canonical_triple())
            .ok_or_else(|| error("Swift package production requires a registered target"))?;
        let architecture = scoop_target_data::apple_architecture(target.architecture);
        let mut result = Vec::new();
        for library in libraries {
            let framework = library.extension().is_some_and(|ext| ext == "framework");
            let name = library
                .file_stem()
                .and_then(|name| name.to_str())
                .ok_or_else(|| error("invalid Swift companion filename"))?;
            let name = if framework {
                name
            } else {
                name.strip_prefix("lib")
                    .ok_or_else(|| error("Swift companion filename must start with lib"))?
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
            result.push(crate::NativeLibraryDependency {
                name,
                linkage,
                framework,
                library,
                binary,
            });
        }
        Ok(result)
    }
}

fn file_path(value: &str) -> Option<PathBuf> {
    // Swift Build file-copy nodes append '/' even to regular resource files.
    let path = PathBuf::from(value.trim_end_matches('/'));
    if !path.is_absolute() {
        return None;
    }
    Some(path.canonicalize().unwrap_or(path))
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
