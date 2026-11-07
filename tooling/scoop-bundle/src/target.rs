use anyhow::{Result, bail};
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct Target {
    #[serde(rename = "canonical")]
    pub triple: String,
    pub os: String,
    pub arch: String,
    pub env: String,
}

pub fn catalog() -> &'static [Target] {
    static CATALOG: std::sync::LazyLock<Vec<Target>> = std::sync::LazyLock::new(|| {
        #[derive(Deserialize)]
        struct Catalog {
            targets: Vec<Target>,
        }
        toml::from_str::<Catalog>(include_str!("../../../compiler/target-data/targets.toml"))
            .expect("built-in target catalog must be valid")
            .targets
    });
    &CATALOG
}

impl Target {
    pub fn parse(triple: &str) -> Result<Self> {
        catalog()
            .iter()
            .find(|target| target.triple == triple)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("unknown canonical Scoop target {triple:?}"))
    }

    pub fn require_host(&self) -> Result<()> {
        let host = if cfg!(target_os = "macos") {
            "darwin"
        } else {
            std::env::consts::OS
        };
        if self.os != host {
            bail!(
                "target {} must be packaged on {}, current host is {host}",
                self.triple,
                self.os
            );
        }
        Ok(())
    }
}
