use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Macos {
    #[serde(default)]
    pub formats: Vec<String>,
    pub short_version: Option<String>,
    pub build_version: Option<String>,
    pub minimum_system_version: Option<String>,
    pub info_plist: Option<PathBuf>,
    #[serde(default)]
    pub signing: MacSigning,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct MacSigning {
    #[serde(default)]
    pub mode: SigningMode,
    pub identity: Option<String>,
    pub entitlements: Option<PathBuf>,
    pub hardened_runtime: Option<bool>,
    pub timestamp: Option<bool>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum SigningMode {
    #[default]
    AdHoc,
    Identity,
    None,
}
