use super::*;

#[derive(Serialize, Deserialize)]
struct Copied {
    #[serde(with = "values::path")]
    source: PathBuf,
    #[serde(with = "values::paths")]
    required: Vec<PathBuf>,
}

pub(super) fn publish(
    source: &Path,
    target: &Path,
    cache: &Path,
) -> Result<(Vec<PathBuf>, Vec<Input>), ToolchainError> {
    let key = build_record::fingerprint(
        &scoop_protocol::HostPathCarrier::from_path(target).map_err(error)?,
    )?;
    let record = build_record::cached(
        &cache.join("native-copies-v1"),
        key,
        |copy: &Copied| Ok(copy.source == source && available(&copy.required)?),
        || {
            let parent = target
                .parent()
                .ok_or_else(|| error("native output has no directory"))?;
            std::fs::create_dir_all(parent).map_err(error)?;
            let directory = tempfile::Builder::new()
                .prefix(".native-output-")
                .tempdir_in(parent)
                .map_err(error)?;
            let candidate = directory.path().join("candidate");
            let mut inputs = Vec::new();
            let mut paths = Vec::new();
            copy(source, &candidate, &mut inputs, &mut paths)?;
            let required = paths
                .iter()
                .map(|path| {
                    let relative = path
                        .strip_prefix(&candidate)
                        .expect("copied path belongs to candidate");
                    if relative.as_os_str().is_empty() {
                        target.to_owned()
                    } else {
                        target.join(relative)
                    }
                })
                .collect();
            let old = directory.path().join("previous");
            let previous = match std::fs::rename(target, &old) {
                Ok(()) => true,
                Err(cause) if cause.kind() == std::io::ErrorKind::NotFound => false,
                Err(cause) => return Err(error(cause)),
            };
            if let Err(cause) = std::fs::rename(&candidate, target) {
                if previous {
                    std::fs::rename(&old, target).map_err(error)?;
                }
                return Err(error(cause));
            }
            Ok(Record {
                inputs,
                result: Copied {
                    source: source.to_owned(),
                    required,
                },
            })
        },
    )?;
    Ok((record.result.required, record.inputs))
}

fn copy(
    source: &Path,
    target: &Path,
    inputs: &mut Vec<Input>,
    required: &mut Vec<PathBuf>,
) -> Result<(), ToolchainError> {
    inputs.push(Input::read(source)?);
    let metadata = source.symlink_metadata().map_err(error)?;
    if metadata.file_type().is_symlink() {
        #[cfg(unix)]
        std::os::unix::fs::symlink(std::fs::read_link(source).map_err(error)?, target)
            .map_err(error)?;
        #[cfg(not(unix))]
        return Err(error("Swift native publication requires macOS"));
    } else if metadata.is_dir() {
        std::fs::create_dir(target).map_err(error)?;
        for entry in std::fs::read_dir(source).map_err(error)? {
            let entry = entry.map_err(error)?;
            copy(
                &entry.path(),
                &target.join(entry.file_name()),
                inputs,
                required,
            )?;
        }
    } else {
        std::fs::copy(source, target).map_err(error)?;
    }
    required.push(target.to_owned());
    Ok(())
}
