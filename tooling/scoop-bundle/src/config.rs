mod platform;
pub use platform::*;

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};

use crate::target::{Target, catalog};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Config {
    pub schema: u32,
    pub identifier: String,
    pub name: Option<String>,
    pub version: Option<String>,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub license: Option<String>,
    pub repository: Option<String>,
    pub homepage: Option<String>,
    pub publisher: Option<String>,
    #[serde(default)]
    pub icons: Vec<PathBuf>,
    #[serde(default)]
    pub files: Vec<FileMapping>,
    #[serde(default)]
    pub library_search_paths: Vec<PathBuf>,
    #[serde(default)]
    pub macos: Macos,
    #[serde(default)]
    pub tools: BTreeMap<String, PathBuf>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum FileKind {
    Resource,
    Executable,
    Library,
    Framework,
    File,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct FileMapping {
    pub from: PathBuf,
    pub to: String,
    pub kind: FileKind,
    #[serde(default)]
    pub when: BTreeMap<String, PredicateValues>,
    #[serde(default)]
    pub macos: FileMacos,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FileMacos {
    pub entitlements: Option<PathBuf>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum PredicateValues {
    One(String),
    Many(Vec<String>),
}

impl PredicateValues {
    fn values(&self) -> &[String] {
        match self {
            Self::One(value) => std::slice::from_ref(value),
            Self::Many(values) => values,
        }
    }
}

fn fact<'a>(target: &'a Target, key: &str) -> Result<&'a str> {
    match key {
        "os" => Ok(&target.os),
        "arch" => Ok(&target.arch),
        "env" => Ok(&target.env),
        _ => bail!("unknown target condition {key:?}; expected os, arch or env"),
    }
}

impl FileMapping {
    pub fn matches(&self, target: &Target) -> bool {
        self.when.iter().all(|(key, values)| {
            values
                .values()
                .iter()
                .any(|v| fact(target, key).is_ok_and(|f| f == v))
        })
    }
}

pub fn relative_destination(text: &str) -> Result<PathBuf> {
    ensure!(
        !text.is_empty() && !text.starts_with('/') && !text.starts_with('\\'),
        "destination must be a nonempty relative path"
    );
    let mut path = PathBuf::new();
    for part in text.split(['/', '\\']) {
        match part {
            "" | "." => (),
            ".." => {
                ensure!(
                    path.pop(),
                    "destination escapes its package directory: {text}"
                );
            }
            _ => {
                ensure!(
                    !part.contains(':') && !part.contains('\0'),
                    "invalid destination component {part:?}"
                );
                path.push(part);
            }
        }
    }
    ensure!(
        !path.as_os_str().is_empty(),
        "destination must name a file or directory"
    );
    Ok(path)
}

pub fn file_name(value: &str, field: &str) -> Result<()> {
    ensure!(
        !value.trim().is_empty()
            && !matches!(value, "." | "..")
            && !value.contains(['/', '\\', ':', '\0'])
            && !value.chars().any(char::is_control),
        "bundle.{field} must be a single nonempty file name"
    );
    Ok(())
}

#[derive(Debug)]
pub struct Loaded {
    pub config: Config,
    pub source: PathBuf,
    pub base: PathBuf,
    pub name: String,
    pub version: semver::Version,
    pub display_name: String,
}

impl Loaded {
    pub fn path(&self, path: &Path) -> PathBuf {
        self.base.join(path)
    }

    pub fn finish(
        mut config: Config,
        source: PathBuf,
        name: Option<&str>,
        version: Option<&str>,
        metadata: &BTreeMap<String, String>,
    ) -> Result<Self> {
        ensure!(config.schema == 1, "bundle.schema must be 1");
        let id = &config.identifier;
        ensure!(
            id.split('.').count() >= 2
                && id.split('.').all(|p| !p.is_empty()
                    && p.starts_with(|c: char| c.is_ascii_alphabetic())
                    && p.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')),
            "bundle.identifier must be a reverse-domain application identifier"
        );
        let name = config
            .name
            .clone()
            .or_else(|| name.map(str::to_owned))
            .context("bundle.name is required for an existing executable")?;
        file_name(&name, "name")?;
        let version = config
            .version
            .as_deref()
            .or(version)
            .context("bundle.version is required for an existing executable")?
            .parse()
            .context("bundle.version must be an exact SemVer")?;
        let display_name = config.display_name.clone().unwrap_or_else(|| name.clone());
        file_name(&display_name, "display-name")?;
        for (value, key) in [
            (&mut config.description, "description"),
            (&mut config.license, "license"),
            (&mut config.repository, "repository"),
        ] {
            if value.is_none() {
                *value = metadata.get(key).cloned();
            }
        }
        for (index, item) in config.files.iter().enumerate() {
            relative_destination(&item.to).with_context(|| format!("bundle.files[{index}].to"))?;
            for (key, values) in &item.when {
                for value in values.values() {
                    ensure!(
                        catalog()
                            .iter()
                            .any(|target| fact(target, key).is_ok_and(|f| f == value)),
                        "bundle.files[{index}].when: unknown {key} value {value:?}"
                    );
                }
                fact(&catalog()[0], key)?;
            }
            ensure!(
                item.macos.entitlements.is_none() || item.kind == FileKind::Executable,
                "bundle.files[{index}].macos.entitlements requires kind = executable"
            );
        }
        let base = source
            .parent()
            .context("configuration has no parent directory")?
            .to_path_buf();
        Ok(Self {
            config,
            source,
            base,
            name,
            version,
            display_name,
        })
    }
}

pub fn parse_external(path: &Path) -> Result<Config> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Document {
        bundle: Config,
    }
    let text = std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    toml::from_str::<Document>(&text)
        .map(|doc| doc.bundle)
        .with_context(|| format!("{}: invalid bundle configuration", path.display()))
}
