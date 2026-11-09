//! Cache dependency headers by normal input update metadata before opening source text.
use std::fs::File;
use std::io::{BufReader, BufWriter, ErrorKind, Write};
use std::path::Path;
use std::time::Duration;

use scoop_manifest::{
    SingleFileDependency, SingleFileInputError, SingleFileInputIoOperation as Operation,
    SingleFileLocator, load_single_file_source, parse_single_file_dependencies,
    select_single_file_source,
};
use scoop_wire::sha256;

use crate::ArtifactCacheRoot;

#[cfg(test)]
mod tests;

#[derive(serde::Serialize, serde::Deserialize)]
struct HeaderRecord {
    modified: (bool, Duration),
    byte_length: u64,
    dependencies: Vec<SingleFileDependency>,
}

impl ArtifactCacheRoot {
    pub(crate) fn load_single_file_header(
        &self,
        locator: &SingleFileLocator,
    ) -> Result<Vec<SingleFileDependency>, SingleFileInputError> {
        let source = select_single_file_source(locator)?;
        let modified = match source.modified().duration_since(std::time::UNIX_EPOCH) {
            Ok(duration) => (false, duration),
            Err(error) => (true, error.duration()),
        };
        let byte_length = source.byte_length();
        let directory = self.as_path().join("single-file-headers-v2");
        let name = sha256(source.physical_path().as_os_str().as_encoded_bytes()).to_string();
        let record = directory.join(format!("{name}.json"));
        if let Some(header) = read_record(&record, modified, byte_length)? {
            return Ok(header);
        }
        std::fs::create_dir_all(&directory)
            .map_err(|error| io_error(&directory, Operation::Write, error))?;
        let lock_path = directory.join(format!("{name}.lock"));
        let lock = super::open_build_lock(&lock_path)
            .map_err(|error| io_error(&lock_path, Operation::Lock, error))?;
        fs4::FileExt::lock(&lock).map_err(|error| io_error(&lock_path, Operation::Lock, error))?;
        if let Some(header) = read_record(&record, modified, byte_length)? {
            return Ok(header);
        }
        let source = load_single_file_source(locator)?;
        let dependencies =
            parse_single_file_dependencies(source.source_text()).map_err(|error| {
                SingleFileInputError::header(locator.display_path().to_owned(), error)
            })?;
        let mut output = tempfile::NamedTempFile::new_in(&directory)
            .map_err(|error| io_error(&record, Operation::Write, error))?;
        let header = HeaderRecord {
            modified,
            byte_length,
            dependencies,
        };
        {
            let mut writer = BufWriter::new(output.as_file_mut());
            serde_json::to_writer(&mut writer, &header)
                .map_err(|error| io_error(&record, Operation::Write, error.into()))?;
            writer
                .flush()
                .map_err(|error| io_error(&record, Operation::Write, error))?;
        }
        output
            .persist(&record)
            .map_err(|error| io_error(&record, Operation::Write, error.error))?;
        Ok(header.dependencies)
    }
}

fn read_record(
    path: &Path,
    modified: (bool, Duration),
    byte_length: u64,
) -> Result<Option<Vec<SingleFileDependency>>, SingleFileInputError> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(io_error(path, Operation::Read, error)),
    };
    let header: HeaderRecord = serde_json::from_reader(BufReader::new(file))
        .map_err(|error| io_error(path, Operation::Read, error.into()))?;
    Ok(
        (header.modified == modified && header.byte_length == byte_length)
            .then_some(header.dependencies),
    )
}

fn io_error(path: &Path, operation: Operation, error: std::io::Error) -> SingleFileInputError {
    SingleFileInputError::io(operation, path.to_owned(), error)
}
