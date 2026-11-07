use std::path::PathBuf;

use clap::{Parser, ValueEnum};

#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub enum MessageFormat {
    #[default]
    Human,
    Json,
}

#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub enum Profile {
    Debug,
    #[default]
    Release,
}

impl Profile {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Debug => "debug",
            Self::Release => "release",
        }
    }
}

#[derive(Debug, Parser)]
#[command(version, about = "Package a Scoop executable and its resources")]
pub struct Args {
    /// Cone directory or Cone.toml; defaults to the current directory.
    #[arg(conflicts_with = "executable")]
    pub root_input: Option<PathBuf>,
    /// Select exactly one executable member by group:name.
    #[arg(short = 'p', long = "package", conflicts_with = "executable")]
    pub package: Option<String>,
    /// Package an existing binary, without invoking Scoop or PM.
    #[arg(long, requires_all = ["config", "target"], conflicts_with_all = ["profile", "scoop", "package_manager", "sysroot", "locked", "offline", "frozen", "jobs"])]
    pub executable: Option<PathBuf>,
    /// Select a complete external [bundle] configuration.
    #[arg(long)]
    pub config: Option<PathBuf>,
    #[arg(long = "format")]
    pub formats: Vec<String>,
    #[arg(long, value_enum)]
    pub profile: Option<Profile>,
    #[arg(long)]
    pub target: Option<String>,
    #[arg(long)]
    pub target_dir: Option<PathBuf>,
    #[arg(long)]
    pub output_dir: Option<PathBuf>,
    #[arg(long)]
    pub scoop: Option<PathBuf>,
    #[arg(long)]
    pub package_manager: Option<PathBuf>,
    #[arg(long)]
    pub sysroot: Option<PathBuf>,
    #[arg(long)]
    pub locked: bool,
    #[arg(long)]
    pub offline: bool,
    #[arg(long)]
    pub frozen: bool,
    #[arg(short = 'j', long)]
    pub jobs: Option<std::num::NonZeroUsize>,
    /// Repackage and sign even when the inputs have not changed.
    #[arg(long)]
    pub force: bool,
    #[arg(long, value_enum, default_value_t)]
    pub message_format: MessageFormat,
}

impl Args {
    pub fn json(&self) -> bool {
        matches!(self.message_format, MessageFormat::Json)
    }
}
