//! Ordinary metadata update records for native sources and objects.
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter, Write};
use std::path::Path;

use scoop_wire::{Digest256, sha256};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::ToolchainError;
pub(crate) mod inputs;
pub(crate) use inputs::Input;
pub(crate) mod values;

#[derive(Serialize, Deserialize)]
pub(crate) struct Record<T> {
    pub(crate) inputs: Vec<Input>,
    pub(crate) result: T,
}

pub(crate) fn lookup<T: DeserializeOwned>(
    path: &Path,
    available: impl Fn(&T) -> Result<bool, ToolchainError>,
) -> Result<Option<Record<T>>, ToolchainError> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(cause) => return Err(error(scoop_source::DiagnosticCode::E2201, cause)),
    };
    let record: Record<T> = serde_json::from_reader(BufReader::new(file))
        .map_err(|cause| error(scoop_source::DiagnosticCode::ICE2201, cause))?;
    for input in &record.inputs {
        if !input.is_current()? {
            return Ok(None);
        }
    }
    Ok(available(&record.result)?.then_some(record))
}

pub(crate) fn cached<T: Serialize + DeserializeOwned>(
    directory: &Path,
    key: Digest256,
    available: impl Fn(&T) -> Result<bool, ToolchainError>,
    build: impl FnOnce() -> Result<Record<T>, ToolchainError>,
) -> Result<Record<T>, ToolchainError> {
    let path = directory.join(format!("{key}.json"));
    if let Some(record) = lookup(&path, &available)? {
        return Ok(record);
    }
    std::fs::create_dir_all(directory)
        .map_err(|cause| error(scoop_source::DiagnosticCode::E2201, cause))?;
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(directory.join(format!("{key}.lock")))
        .map_err(|cause| error(scoop_source::DiagnosticCode::E2201, cause))?;
    fs4::FileExt::lock(&lock).map_err(|cause| error(scoop_source::DiagnosticCode::E2201, cause))?;
    if let Some(record) = lookup(&path, &available)? {
        return Ok(record);
    }
    let record = build()?;
    // tempfile passes both paths directly to the native Windows rename API.
    #[cfg(windows)]
    let directory = std::fs::canonicalize(directory)
        .map_err(|cause| error(scoop_source::DiagnosticCode::E2201, cause))?;
    #[cfg(windows)]
    let directory = directory.as_path();
    #[cfg(windows)]
    let path = directory.join(format!("{key}.json"));
    let mut file = tempfile::NamedTempFile::new_in(directory)
        .map_err(|cause| error(scoop_source::DiagnosticCode::E2201, cause))?;
    {
        let mut writer = BufWriter::new(&mut file);
        serde_json::to_writer(&mut writer, &record)
            .map_err(|cause| error(scoop_source::DiagnosticCode::ICE2201, cause))?;
        writer
            .flush()
            .map_err(|cause| error(scoop_source::DiagnosticCode::E2201, cause))?;
    }
    file.persist(path)
        .map_err(|e| error(scoop_source::DiagnosticCode::E2201, e.error))?;
    Ok(record)
}

pub(crate) fn fingerprint(value: &impl Serialize) -> Result<Digest256, ToolchainError> {
    Ok(sha256(&serde_json::to_vec(value).map_err(|cause| {
        error(scoop_source::DiagnosticCode::ICE2201, cause)
    })?))
}

pub(crate) fn error(
    code: scoop_source::DiagnosticCode,
    value: impl std::fmt::Display,
) -> ToolchainError {
    ToolchainError::new(code, format!("native build record: {value}"))
}

#[cfg(all(test, windows))]
mod tests;
