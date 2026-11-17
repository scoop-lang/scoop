use super::*;
use scoop_protocol::HostPathCarrier;
use serde::{Deserialize, Serialize};
use std::io::{BufReader, BufWriter, Write};

#[derive(Serialize, Deserialize)]
struct Record {
    inputs: Vec<NativeInputMetadata>,
    executable: HostPathCarrier,
    runtime_index: HostPathCarrier,
    import_library: Option<HostPathCarrier>,
    debug_file: Option<(scoop_toolchain::DebugFileKind, HostPathCarrier)>,
    fingerprint: [u8; 32],
    result_key: [u8; 32],
    native_outputs: Vec<scoop_toolchain::NativeLibraryOutput>,
}

pub(super) fn load(path: &Path) -> BuildResult<Option<Executable>> {
    let file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(failure(scoop_source::DiagnosticCode::E2201, error)),
    };
    let record: Record = serde_json::from_reader(BufReader::new(file))
        .map_err(|error| failure(scoop_source::DiagnosticCode::ICE2201, error))?;
    for input in &record.inputs {
        if !input
            .is_current()
            .map_err(|error| failure(error.code(), error))?
        {
            return Ok(None);
        }
    }
    let path = record
        .executable
        .to_path_buf()
        .map_err(|error| failure(scoop_source::DiagnosticCode::ICE2201, error))?;
    if !path
        .try_exists()
        .map_err(|error| failure(scoop_source::DiagnosticCode::E2201, error))?
    {
        return Ok(None);
    }
    let debug_file = record
        .debug_file
        .map(|(kind, path)| {
            path.to_path_buf()
                .map(|path| scoop_linker::DebugFile { kind, path })
                .map_err(|error| failure(scoop_source::DiagnosticCode::ICE2201, error))
        })
        .transpose()?;
    if let Some(file) = &debug_file
        && !file
            .path
            .try_exists()
            .map_err(|error| failure(scoop_source::DiagnosticCode::E2201, error))?
    {
        return Ok(None);
    }
    let import_library = record
        .import_library
        .map(|path| {
            path.to_path_buf()
                .map_err(|error| failure(scoop_source::DiagnosticCode::ICE2201, error))
        })
        .transpose()?;
    if let Some(path) = &import_library
        && !path
            .try_exists()
            .map_err(|error| failure(scoop_source::DiagnosticCode::E2201, error))?
    {
        return Ok(None);
    }
    Ok(Some(Executable {
        native_outputs: record.native_outputs,
        path,
        runtime_index: record
            .runtime_index
            .to_path_buf()
            .map_err(|error| failure(scoop_source::DiagnosticCode::ICE2201, error))?,
        debug_file,
        import_library,
        fingerprint: ResolvedLinkPlanFingerprint::from_array(record.fingerprint),
        result_key: record.result_key,
    }))
}

pub(super) fn save(
    path: &Path,
    executable: &Executable,
    inputs: Vec<NativeInputMetadata>,
) -> BuildResult<()> {
    let record = Record {
        native_outputs: executable.native_outputs.clone(),
        inputs,
        import_library: executable
            .import_library
            .as_deref()
            .map(HostPathCarrier::from_path)
            .transpose()
            .map_err(|error| failure(scoop_source::DiagnosticCode::ICE2201, error))?,
        executable: HostPathCarrier::from_path(&executable.path)
            .map_err(|error| failure(scoop_source::DiagnosticCode::ICE2201, error))?,
        runtime_index: HostPathCarrier::from_path(&executable.runtime_index)
            .map_err(|error| failure(scoop_source::DiagnosticCode::ICE2201, error))?,
        fingerprint: *executable.fingerprint.as_array(),
        debug_file: executable
            .debug_file
            .as_ref()
            .map(|file| {
                HostPathCarrier::from_path(&file.path)
                    .map(|path| (file.kind, path))
                    .map_err(|error| failure(scoop_source::DiagnosticCode::ICE2201, error))
            })
            .transpose()?,
        result_key: executable.result_key,
    };
    let mut file = tempfile::NamedTempFile::new_in(path.parent().unwrap())
        .map_err(|error| failure(scoop_source::DiagnosticCode::E2201, error))?;
    {
        let mut writer = BufWriter::new(&mut file);
        serde_json::to_writer(&mut writer, &record)
            .map_err(|error| failure(scoop_source::DiagnosticCode::ICE2201, error))?;
        writer
            .flush()
            .map_err(|error| failure(scoop_source::DiagnosticCode::E2201, error))?;
    }
    file.persist(path)
        .map_err(|error| failure(scoop_source::DiagnosticCode::E2201, error.error))?;
    Ok(())
}
