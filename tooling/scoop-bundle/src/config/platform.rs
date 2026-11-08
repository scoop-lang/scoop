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

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Linux {
    #[serde(default)]
    pub formats: Vec<String>,
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub terminal: bool,
    #[serde(default)]
    pub actions: Vec<DesktopAction>,
    #[serde(default)]
    pub deb: Deb,
    #[serde(default)]
    pub rpm: Rpm,
    #[serde(default)]
    pub appimage: AppImage,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct DesktopAction {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub arguments: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Deb {
    pub maintainer: Option<String>,
    pub section: Option<String>,
    pub version: Option<String>,
    pub revision: Option<String>,
    #[serde(default)]
    pub depends: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Rpm {
    pub version: Option<String>,
    pub release: Option<String>,
    #[serde(default)]
    pub requires: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct AppImage {
    #[serde(default)]
    pub system_libraries: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Windows {
    #[serde(default)]
    pub formats: Vec<String>,
    #[serde(default)]
    pub install_mode: InstallMode,
    pub vc_runtime: Option<VcRuntime>,
    #[serde(default)]
    pub nsis: Nsis,
    #[serde(default)]
    pub msi: Msi,
    pub signing: Option<WindowsSigning>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum InstallMode {
    #[default]
    PerUser,
    PerMachine,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum VcRuntime {
    System,
    AppLocal,
}

fn yes() -> bool {
    true
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Nsis {
    pub file_version: Option<String>,
    #[serde(default = "yes")]
    pub start_menu_shortcut: bool,
    #[serde(default)]
    pub desktop_shortcut: bool,
}

impl Default for Nsis {
    fn default() -> Self {
        Self {
            file_version: None,
            start_menu_shortcut: true,
            desktop_shortcut: false,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Msi {
    pub upgrade_code: Option<String>,
    pub product_version: Option<String>,
    #[serde(default = "yes")]
    pub start_menu_shortcut: bool,
    #[serde(default)]
    pub desktop_shortcut: bool,
}

impl Default for Msi {
    fn default() -> Self {
        Self {
            upgrade_code: None,
            product_version: None,
            start_menu_shortcut: true,
            desktop_shortcut: false,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct WindowsSigning {
    pub certificate_thumbprint: String,
    pub timestamp_url: String,
    #[serde(default)]
    pub certificate_store: CertificateStore,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum CertificateStore {
    #[default]
    CurrentUser,
    LocalMachine,
}
