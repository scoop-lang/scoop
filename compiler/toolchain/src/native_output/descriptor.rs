use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::NativeLibraryOutput;
use crate::ToolchainError;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeOutputsDescriptor {
    pub schema: u32,
    pub outputs: Vec<NativeLibraryOutput>,
}

impl NativeLibraryOutput {
    pub(crate) fn map_paths(
        &mut self,
        mut map: impl FnMut(&Path) -> Result<PathBuf, ToolchainError>,
    ) -> Result<(), ToolchainError> {
        self.library = map(&self.library)?;
        self.binary = map(&self.binary)?;
        for dependency in &mut self.dependencies {
            dependency.library = map(&dependency.library)?;
            dependency.binary = map(&dependency.binary)?;
        }
        for path in self
            .resources
            .iter_mut()
            .chain(&mut self.settings.library_paths)
            .chain(&mut self.settings.framework_paths)
        {
            *path = map(path)?;
        }
        Ok(())
    }
}

pub fn read_native_outputs(
    root: &Path,
) -> Result<Option<Vec<NativeLibraryOutput>>, ToolchainError> {
    let path = root.join("native-outputs.json");
    let file = match std::fs::File::open(&path) {
        Ok(file) => file,
        Err(cause) if cause.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(cause) => return Err(error(format!("{}: {cause}", path.display()))),
    };
    let mut descriptor: NativeOutputsDescriptor =
        serde_json::from_reader(std::io::BufReader::new(file))
            .map_err(|cause| error(format!("{}: {cause}", path.display())))?;
    if descriptor.schema != 1 {
        return Err(error(format!(
            "{}: unsupported native output schema {}",
            path.display(),
            descriptor.schema
        )));
    }
    let mut names = std::collections::BTreeSet::new();
    for output in &mut descriptor.outputs {
        scoop_identity::CanonicalNativeLibraryName::new(&output.name).map_err(error)?;
        if !names.insert(output.name.clone()) {
            return Err(error(format!(
                "{}: duplicate native output {}",
                path.display(),
                output.name
            )));
        }
        for dependency in &output.dependencies {
            scoop_identity::CanonicalNativeLibraryName::new(&dependency.name).map_err(error)?;
        }
        output.map_paths(|relative| {
            if relative.as_os_str().is_empty()
                || relative
                    .components()
                    .any(|c| !matches!(c, Component::Normal(_)))
            {
                return Err(error(format!(
                    "{}: native output path must be a relative file or directory: {}",
                    path.display(),
                    relative.display()
                )));
            }
            Ok(root.join(relative))
        })?;
    }
    Ok(Some(descriptor.outputs))
}

fn error(cause: impl std::fmt::Display) -> ToolchainError {
    ToolchainError::new(scoop_source::DiagnosticCode::E2305, cause.to_string())
}
