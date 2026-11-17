//! Inputs of the selected Swift Build description, including compiler depfiles.
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::Deserialize;

use super::{SwiftLinkage, ToolchainError, error};

#[derive(Deserialize)]
struct Manifest {
    targets: BTreeMap<String, Vec<String>>,
    commands: BTreeMap<String, BuildCommand>,
}

#[derive(Clone, Deserialize)]
pub(super) struct BuildCommand {
    #[serde(default)]
    pub(super) description: String,
    #[serde(default)]
    inputs: Vec<String>,
    #[serde(default)]
    outputs: Vec<String>,
    #[serde(default, rename = "expectedOutputs")]
    expected_outputs: Vec<String>,
    #[serde(default)]
    pub(super) args: Vec<String>,
    #[serde(default)]
    deps: Option<DependencyFiles>,
}

#[derive(Clone, Deserialize)]
#[serde(untagged)]
enum DependencyFiles {
    One(String),
    Many(Vec<String>),
}

impl DependencyFiles {
    fn paths(&self) -> &[String] {
        match self {
            Self::One(path) => std::slice::from_ref(path),
            Self::Many(paths) => paths,
        }
    }
}

pub(super) struct BuildPlan {
    pub(super) inputs: BTreeSet<PathBuf>,
    pub(super) dependencies: BTreeSet<PathBuf>,
    pub(super) outputs: BTreeSet<PathBuf>,
    pub(super) commands: Vec<BuildCommand>,
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
        let mut leaf_inputs = BTreeSet::new();
        while let Some(input) = queue.pop() {
            if let Some(id) = producers.get(input) {
                if !visited.insert(*id) {
                    continue;
                }
                let command = &manifest.commands[*id];
                queue.extend(command.inputs.iter().map(String::as_str));
                commands.push(command.clone());
            } else if let Some(path) = file_path(input) {
                leaf_inputs.insert(path);
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
        let mut depfiles = outputs
            .iter()
            .filter(|p| p.extension().is_some_and(|ext| ext == "d"))
            .cloned()
            .collect::<BTreeSet<_>>();
        for command in &commands {
            depfiles.extend(
                command
                    .deps
                    .iter()
                    .flat_map(DependencyFiles::paths)
                    .filter_map(|path| file_path(path))
                    .filter(|path| path.extension().is_some_and(|ext| ext == "d")),
            );
        }
        let mut dependencies = BTreeSet::new();
        for depfile in depfiles {
            let text = match std::fs::read_to_string(&depfile) {
                Ok(text) => text,
                Err(cause) if cause.kind() == std::io::ErrorKind::NotFound => continue,
                Err(cause) => return Err(error(cause)),
            };
            let text = text.replace("\\\n", " ").replace("\\\r\n", " ");
            for rule in text.lines().filter(|line| !line.trim().is_empty()) {
                let Some((target, _)) = rule.split_once(": ") else {
                    return Err(error(format!(
                        "invalid Swift compiler dependencies in {}",
                        depfile.display()
                    )));
                };
                let Some(paths) = scoop_process::parse_make_dependencies(rule, target) else {
                    continue;
                };
                for path in paths {
                    let path = if path.is_absolute() {
                        path
                    } else {
                        scratch.join(path)
                    };
                    dependencies.insert(path.canonicalize().map_err(error)?);
                }
            }
        }
        Ok(Self {
            inputs: leaf_inputs,
            dependencies,
            outputs,
            commands,
        })
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
