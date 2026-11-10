use super::error;
use crate::ToolchainError;
use scoop_protocol::HostPathCarrier;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

fn path(path: &Path) -> Result<HostPathCarrier, ToolchainError> {
    HostPathCarrier::from_path(path).map_err(error)
}
use std::collections::BTreeSet;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Input {
    path: HostPathCarrier,
    state: Option<State>,
}

#[derive(Clone, Debug, Eq, Serialize, Deserialize, PartialEq)]
struct State {
    resolved: HostPathCarrier,
    length: u64,
    modified: (bool, u64, u32),
}

impl Input {
    pub fn read(input: &Path) -> Result<Self, ToolchainError> {
        Ok(Self {
            path: path(input)?,
            state: state(input)?,
        })
    }

    pub fn is_current(&self) -> Result<bool, ToolchainError> {
        Ok(state(&self.path.to_path_buf().map_err(error)?)? == self.state)
    }

    pub fn path(&self) -> Result<PathBuf, ToolchainError> {
        self.path.to_path_buf().map_err(error)
    }
}

fn state(path: &Path) -> Result<Option<State>, ToolchainError> {
    let metadata = match path.metadata() {
        Ok(value) => value,
        Err(error)
            if matches!(
                error.kind(),
                std::io::ErrorKind::NotFound | std::io::ErrorKind::NotADirectory
            ) =>
        {
            return Ok(None);
        }
        Err(source) => {
            return Err(error(format!(
                "cannot inspect {}: {source}",
                path.display()
            )));
        }
    };
    let (before, elapsed) = match metadata
        .modified()
        .map_err(error)?
        .duration_since(std::time::UNIX_EPOCH)
    {
        Ok(elapsed) => (false, elapsed),
        Err(error) => (true, error.duration()),
    };
    Ok(Some(State {
        resolved: self::path(&path.canonicalize().map_err(error)?)?,
        length: metadata.len(),
        modified: (before, elapsed.as_secs(), elapsed.subsec_nanos()),
    }))
}

/// Observe include search choices and the parents of headers used by the probe.
pub fn native_include_search_paths(stderr: &[u8]) -> Vec<PathBuf> {
    let mut search = false;
    let mut roots = Vec::new();
    for line in String::from_utf8_lossy(stderr).lines() {
        if line.contains("search starts here:") {
            search = true;
            continue;
        }
        if line == "End of search list." {
            search = false;
        }
        if search && line.starts_with(' ') {
            roots.push(PathBuf::from(
                line.trim().trim_end_matches(" (framework directory)"),
            ));
        } else if let Some(path) = line
            .strip_prefix("ignoring nonexistent directory \"")
            .and_then(|v| v.strip_suffix('"'))
        {
            roots.push(PathBuf::from(path));
        }
    }
    roots
}

pub(crate) fn header_candidates(stderr: &[u8], inputs: &mut BTreeSet<PathBuf>) {
    let roots = native_include_search_paths(stderr);
    for header in inputs.clone() {
        if let Some(parent) = header.parent().filter(|p| !p.as_os_str().is_empty()) {
            inputs.insert(parent.to_path_buf());
        }
        for root in &roots {
            if let Ok(relative) = header.strip_prefix(root) {
                for candidate in &roots {
                    inputs.insert(candidate.join(relative));
                }
            }
        }
    }
    inputs.extend(roots);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn observations_detect_real_input_changes_and_new_lookup_candidates() {
        let directory = tempfile::tempdir().unwrap();
        let selected = directory.path().join("selected");
        let earlier = directory.path().join("earlier");
        std::fs::write(&selected, "first").unwrap();
        let input = Input::read(&selected).unwrap();
        let absent = Input::read(&earlier).unwrap();
        assert!(input.is_current().unwrap());
        assert!(absent.is_current().unwrap());
        std::fs::write(&selected, "changed input").unwrap();
        std::fs::write(&earlier, "new candidate").unwrap();
        assert!(!input.is_current().unwrap());
        assert!(!absent.is_current().unwrap());
    }
}
