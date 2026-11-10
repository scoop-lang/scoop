use std::path::{Path, PathBuf};
use std::process::Command;

use scoop_protocol::ScoopcProtocolCapabilityV1;

use crate::PairedScoopcLocator;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedPairedScoopc {
    executable: PathBuf,
}

impl ResolvedPairedScoopc {
    pub fn resolve(locator: &PairedScoopcLocator) -> Self {
        Self {
            executable: locator.as_path().to_path_buf(),
        }
    }

    pub fn executable_path(&self) -> &Path {
        &self.executable
    }

    pub fn protocol(&self) -> ScoopcProtocolCapabilityV1 {
        ScoopcProtocolCapabilityV1::current()
    }
}

pub(crate) fn compiler_command(executable: &Path) -> Command {
    let mut command = Command::new(executable);
    command.env_clear();
    #[cfg(windows)]
    command.envs(
        ["PATH", "SystemRoot"]
            .into_iter()
            .filter_map(|name| std::env::var_os(name).map(|value| (name, value))),
    );
    command
}
