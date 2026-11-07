use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct State {
    kind: u8,
    length: u64,
    modified: Option<(u64, u32)>,
    mode: u32,
}

fn state(path: &Path) -> Result<State> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(State {
                kind: 0,
                length: 0,
                modified: None,
                mode: 0,
            });
        }
        Err(error) => return Err(error).with_context(|| format!("stat {}", path.display())),
    };
    #[cfg(unix)]
    let mode = {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode()
    };
    #[cfg(windows)]
    let mode = u32::from(metadata.permissions().readonly());
    Ok(State {
        kind: if metadata.is_symlink() {
            3
        } else if metadata.is_dir() {
            2
        } else {
            1
        },
        length: metadata.len(),
        modified: metadata
            .modified()?
            .duration_since(UNIX_EPOCH)
            .ok()
            .map(|v| (v.as_secs(), v.subsec_nanos())),
        mode,
    })
}

#[derive(Debug, Deserialize, Serialize)]
struct Watched {
    #[serde(with = "crate::host_path")]
    path: PathBuf,
    state: State,
}

#[derive(Debug, Deserialize, Serialize)]
struct Record {
    schema: u32,
    key: Value,
    #[serde(with = "crate::host_path")]
    artifact: PathBuf,
    inputs: Vec<Watched>,
}

#[derive(Default)]
pub struct Inputs {
    states: BTreeMap<PathBuf, State>,
}

impl Inputs {
    pub fn watch(&mut self, path: &Path) -> Result<()> {
        if !self.states.contains_key(path) {
            self.states.insert(path.into(), state(path)?);
        }
        Ok(())
    }

    pub fn save(self, directory: &Path, key: Value, artifact: PathBuf) -> Result<()> {
        let record = Record {
            schema: 1,
            key,
            artifact,
            inputs: self
                .states
                .into_iter()
                .map(|(path, state)| Watched { path, state })
                .collect(),
        };
        fs::write(
            directory.join(".bundle-state.json"),
            serde_json::to_vec(&record)?,
        )?;
        Ok(())
    }
}

pub fn hit(directory: &Path, key: &Value) -> Result<Option<PathBuf>> {
    let bytes = match fs::read(directory.join(".bundle-state.json")) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let record: Record = match serde_json::from_slice(&bytes) {
        Ok(record) => record,
        Err(_) => return Ok(None),
    };
    if record.schema != 1 || &record.key != key || !directory.join(&record.artifact).exists() {
        return Ok(None);
    }
    for item in record.inputs {
        if state(&item.path)? != item.state {
            return Ok(None);
        }
    }
    Ok(Some(directory.join(record.artifact)))
}
