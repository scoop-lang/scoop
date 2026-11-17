//! Ordinary native files and consumer link settings, independent of production.
use std::collections::BTreeSet;
use std::path::PathBuf;

use scoop_manifest::{SwiftLibraryFormat, SwiftLinkage};
use serde::{Deserialize, Serialize};

use crate::build_record::values;

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
    #[serde(with = "values::paths")]
    pub dependencies: Vec<PathBuf>,
    #[serde(with = "values::paths")]
    pub resources: Vec<PathBuf>,
    pub settings: NativeLinkSettings,
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
