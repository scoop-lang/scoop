//! Batched native archives of selected objects, with no intermediate member files.
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

use ar_archive_writer::{ArchiveKind, DEFAULT_OBJECT_READER, NewArchiveMember};
use fs4::FileExt;
use scoop_lir::{LirTargetProfile, NativeObjectFormat};
use scoop_wire::{Digest256, Encoder, WireEncode, domain_separated_cbor_hash, sha256};

use crate::program::InputObject;
use crate::{LinkError, error};

const MAX_BATCH_BYTES: usize = 64 * 1024 * 1024;
const MAX_BATCH_MEMBERS: usize = 8192;

pub(super) fn build(
    target: LirTargetProfile,
    startup: Option<&[u8]>,
    objects: &[InputObject<'_>],
    cache: &Path,
) -> Result<Vec<PathBuf>, LinkError> {
    let domain = if target.native_object_format() == NativeObjectFormat::MachO64 {
        "scoop-darwin-link-archives-v1"
    } else {
        "scoop-link-archives-v2"
    };
    let key = domain_separated_cbor_hash(
        domain,
        &Members {
            target,
            startup: startup.map(sha256),
            objects,
        },
    )
    .map_err(error)?;
    let namespace = std::path::absolute(cache)
        .map_err(error)?
        .join("link-archives-v2");
    let record = namespace.join(format!("{key}.json"));
    if let Some(paths) = load(&record, &namespace)? {
        return Ok(paths);
    }
    std::fs::create_dir_all(&namespace).map_err(error)?;
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(namespace.join(format!("{key}.lock")))
        .map_err(error)?;
    FileExt::lock(&lock).map_err(error)?;
    if let Some(paths) = load(&record, &namespace)? {
        return Ok(paths);
    }
    let directory = tempfile::Builder::new()
        .prefix("members-")
        .tempdir_in(&namespace)
        .map_err(error)?;
    let (kind, extension) = crate::native_archive::format(target);
    let mut paths = Vec::new();
    let mut members: Vec<_> = startup
        .into_iter()
        .map(|data| NewArchiveMember::new(data, &DEFAULT_OBJECT_READER, "startup.o".to_owned()))
        .collect();
    let mut bytes = startup.map_or(0, <[u8]>::len);
    for (index, object) in objects.iter().enumerate() {
        let data = object.read_bytes(target)?;
        if !members.is_empty()
            && (members.len() >= MAX_BATCH_MEMBERS
                || bytes.saturating_add(data.len()) > MAX_BATCH_BYTES)
        {
            paths.push(write_batch(
                directory.path(),
                key,
                paths.len(),
                extension,
                kind,
                &members,
            )?);
            members.clear();
            bytes = 0;
        }
        bytes = bytes.saturating_add(data.len());
        members.push(NewArchiveMember::new(
            data,
            &DEFAULT_OBJECT_READER,
            format!("{index}.o"),
        ));
    }
    if !members.is_empty() {
        paths.push(write_batch(
            directory.path(),
            key,
            paths.len(),
            extension,
            kind,
            &members,
        )?);
    }
    // Generated relative names are ASCII even when the cache root is not Unicode.
    let names: Vec<_> = paths
        .iter()
        .map(|path| {
            path.strip_prefix(&namespace)
                .expect("archive in cache namespace")
                .to_str()
                .expect("generated archive name")
        })
        .collect();
    let mut file = tempfile::NamedTempFile::new_in(&namespace).map_err(error)?;
    {
        let mut writer = BufWriter::new(file.as_file_mut());
        serde_json::to_writer(&mut writer, &names).map_err(error)?;
        writer.flush().map_err(error)?;
    }
    let _ = directory.keep();
    file.persist(record).map_err(|source| error(source.error))?;
    Ok(paths)
}

fn load(record: &Path, namespace: &Path) -> Result<Option<Vec<PathBuf>>, LinkError> {
    let file = match File::open(record) {
        Ok(file) => file,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(source) => return Err(error(source)),
    };
    let names: Vec<String> = serde_json::from_reader(BufReader::new(file)).map_err(error)?;
    let paths: Vec<_> = names.into_iter().map(|name| namespace.join(name)).collect();
    for path in &paths {
        if !path.try_exists().map_err(error)? {
            return Ok(None);
        }
    }
    Ok(Some(paths))
}

fn write_batch(
    directory: &Path,
    key: Digest256,
    index: usize,
    extension: &str,
    kind: ArchiveKind,
    members: &[NewArchiveMember<'_>],
) -> Result<PathBuf, LinkError> {
    let path = directory.join(format!("part-{key}-{index:04}.{extension}"));
    crate::native_archive::write(&path, kind, members)?;
    Ok(path)
}

struct Members<'objects, 'input> {
    target: LirTargetProfile,
    startup: Option<Digest256>,
    objects: &'objects [InputObject<'input>],
}

impl WireEncode for Members<'_, '_> {
    fn encode(&self, encoder: &mut Encoder) -> Result<(), scoop_wire::cbor::EncodeError> {
        encoder.array(3)?;
        self.target.wire_id().encode(encoder)?;
        encoder.array(u64::from(self.startup.is_some()))?;
        if let Some(startup) = self.startup {
            startup.encode(encoder)?;
        }
        encoder.array(self.objects.len() as u64)?;
        for object in self.objects {
            object.digest().encode(encoder)?;
        }
        Ok(())
    }
}
