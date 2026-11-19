use std::fs;
use std::io;
use std::path::Path;

use scoop_protocol::{
    EmittedDumpDescriptorV1, EmittedDumpDestinationV1, HostPathCarrier, StageDumpPolicyV1,
};

pub(super) fn write_dumps(
    policy: &StageDumpPolicyV1,
    dumps: &[scoopc::EmittedStageDump],
) -> io::Result<Vec<EmittedDumpDescriptorV1>> {
    let (stages, directory) = match policy {
        StageDumpPolicyV1::None if dumps.is_empty() => return Ok(Vec::new()),
        StageDumpPolicyV1::None => return Err(io::Error::other("unexpected stage dumps")),
        StageDumpPolicyV1::Files { stages, directory } => (stages, directory),
    };
    if !stages
        .iter()
        .eq(dumps.iter().map(scoopc::EmittedStageDump::kind))
    {
        return Err(io::Error::other(
            "compiler did not capture the requested stages",
        ));
    }
    let directory = directory.to_path_buf().map_err(io::Error::other)?;
    write_files(&directory, dumps)?;
    dumps
        .iter()
        .map(|dump| {
            let path = directory.join(dump.kind().file_name());
            Ok(EmittedDumpDescriptorV1::new(
                dump.kind(),
                EmittedDumpDestinationV1::File(
                    HostPathCarrier::from_path(&path).map_err(io::Error::other)?,
                ),
            ))
        })
        .collect()
}

pub(super) fn write_files(directory: &Path, dumps: &[scoopc::EmittedStageDump]) -> io::Result<()> {
    fs::create_dir_all(directory)?;
    for dump in dumps {
        fs::write(directory.join(dump.kind().file_name()), dump.text())?;
    }
    Ok(())
}
