//! Ordinary native files and consumer link settings, independent of production.
use std::collections::BTreeSet;
use std::path::PathBuf;

use scoop_manifest::{SwiftLibraryFormat, SwiftLinkage};
use serde::{Deserialize, Serialize};

use crate::build_record::values;

mod publish;
pub use publish::publish_native_outputs;
mod descriptor;
pub use descriptor::{NativeOutputsDescriptor, read_native_outputs};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct NativeLinkSettings {
    pub libraries: BTreeSet<String>,
    pub frameworks: BTreeSet<String>,
    #[serde(with = "values::paths")]
    pub library_paths: Vec<PathBuf>,
    #[serde(with = "values::paths")]
    pub framework_paths: Vec<PathBuf>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NativeLibraryOutput {
    pub name: String,
    pub linkage: SwiftLinkage,
    pub format: SwiftLibraryFormat,
    /// The library file or the enclosing framework directory.
    #[serde(with = "values::path")]
    pub library: PathBuf,
    #[serde(with = "values::path")]
    pub binary: PathBuf,
    pub dependencies: Vec<NativeLibraryDependency>,
    #[serde(with = "values::paths")]
    pub resources: Vec<PathBuf>,
    pub settings: NativeLinkSettings,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NativeLibraryDependency {
    pub name: String,
    pub linkage: SwiftLinkage,
    pub framework: bool,
    #[serde(with = "values::path")]
    pub library: PathBuf,
    #[serde(with = "values::path")]
    pub binary: PathBuf,
}

impl NativeLibraryDependency {
    pub fn dylib(path: PathBuf) -> Result<Self, crate::ToolchainError> {
        let name = path
            .file_stem()
            .and_then(|n| n.to_str())
            .and_then(|n| n.strip_prefix("lib"))
            .ok_or_else(|| {
                crate::ToolchainError::new(
                    scoop_source::DiagnosticCode::E2305,
                    format!("invalid native dylib filename {}", path.display()),
                )
            })?
            .to_owned();
        Ok(Self {
            name,
            linkage: SwiftLinkage::Dynamic,
            framework: false,
            library: path.clone(),
            binary: path,
        })
    }
    pub fn kind(&self) -> scoop_identity::NativeLibraryKind {
        use scoop_identity::NativeLibraryKind;
        if self.framework {
            NativeLibraryKind::Framework
        } else if self.linkage == SwiftLinkage::Static {
            NativeLibraryKind::StaticArchive
        } else {
            NativeLibraryKind::Dynamic
        }
    }
}

impl NativeLibraryOutput {
    pub fn is_static(&self) -> bool {
        self.linkage == SwiftLinkage::Static
    }

    pub fn kind(&self) -> scoop_identity::NativeLibraryKind {
        use scoop_identity::NativeLibraryKind;
        match (&self.format, self.linkage) {
            (SwiftLibraryFormat::Framework { .. }, _) => NativeLibraryKind::Framework,
            (_, SwiftLinkage::Static) => NativeLibraryKind::StaticArchive,
            (_, SwiftLinkage::Dynamic) => NativeLibraryKind::Dynamic,
        }
    }
}
