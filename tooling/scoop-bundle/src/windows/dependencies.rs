use std::{
    collections::BTreeSet,
    fs,
    io::Read,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, ensure};
use goblin::pe::{PE, export::Reexport};

use crate::{cache::Inputs, config::VcRuntime, deploy::Plan, project::Input};

struct Code {
    source: PathBuf,
    destination: PathBuf,
    libraries: Vec<String>,
    executable: bool,
    library: bool,
}

fn delay_imports(bytes: &[u8], pe: &PE<'_>, libraries: &mut Vec<String>) -> Result<()> {
    let header = pe
        .header
        .optional_header
        .as_ref()
        .context("missing PE optional header")?;
    let Some(directory) = header.data_directories.get_delay_import_descriptor() else {
        return Ok(());
    };
    let offset = |rva| {
        goblin::pe::utils::find_offset(
            rva,
            &pe.sections,
            header.windows_fields.file_alignment,
            &Default::default(),
        )
        .context("invalid PE delay-import RVA")
    };
    let start = offset(directory.virtual_address as usize)?;
    let end = start
        .checked_add(directory.size as usize)
        .context("PE delay-import size overflow")?;
    let descriptors = bytes
        .get(start..end)
        .context("truncated PE delay-import directory")?;
    for descriptor in descriptors.as_chunks::<32>().0 {
        if descriptor.iter().all(|v| *v == 0) {
            return Ok(());
        }
        let read = |index| {
            u32::from_le_bytes(
                descriptor[index..index + 4]
                    .try_into()
                    .expect("fixed descriptor field"),
            )
        };
        let attributes = read(0);
        let name = u64::from(read(4));
        let rva = if attributes & 1 == 0 {
            name.checked_sub(pe.image_base)
                .context("invalid PE delay-import address")?
        } else {
            name
        };
        let start = offset(usize::try_from(rva)?)?;
        let bytes = bytes
            .get(start..)
            .context("truncated PE delay-import name")?;
        let end = bytes
            .iter()
            .position(|b| *b == 0)
            .context("unterminated PE delay-import name")?;
        libraries.push(
            std::str::from_utf8(&bytes[..end])
                .context("PE library name must be UTF-8")?
                .to_owned(),
        );
    }
    anyhow::bail!("unterminated PE delay-import directory")
}

fn inspect(source: &Path, destination: &Path) -> Result<Option<Code>> {
    let mut magic = [0; 2];
    if fs::File::open(source)?.read(&mut magic)? != 2 || &magic != b"MZ" {
        return Ok(None);
    }
    let bytes = fs::read(source)?;
    let Some(address) = bytes.get(0x3c..0x40) else {
        return Ok(None);
    };
    let address = u32::from_le_bytes(address.try_into().expect("fixed DOS header field")) as usize;
    if address
        .checked_add(4)
        .and_then(|end| bytes.get(address..end))
        != Some(b"PE\0\0")
    {
        return Ok(None);
    }
    let pe = PE::parse(&bytes).with_context(|| format!("read PE {}", source.display()))?;
    ensure!(
        pe.is_64 && pe.header.coff_header.machine == goblin::pe::header::COFF_MACHINE_X86_64,
        "{} is not an x86_64 PE image",
        source.display()
    );
    let mut libraries = pe
        .libraries
        .iter()
        .map(|s| (*s).to_owned())
        .collect::<Vec<_>>();
    delay_imports(&bytes, &pe, &mut libraries)?;
    for export in &pe.exports {
        if let Some(Reexport::DLLName { lib, .. } | Reexport::DLLOrdinal { lib, .. }) =
            &export.reexport
        {
            libraries.push(if lib.to_ascii_lowercase().ends_with(".dll") {
                (*lib).to_owned()
            } else {
                format!("{lib}.dll")
            });
        }
    }
    libraries.sort_by_key(|name| name.to_lowercase());
    libraries.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
    for name in &libraries {
        ensure!(
            !name.is_empty()
                && !name.contains(['/', '\\', ':'])
                && !name.chars().any(char::is_control),
            "{} has invalid PE library name {name:?}",
            source.display()
        );
    }
    Ok(Some(Code {
        source: source.into(),
        destination: destination.into(),
        libraries,
        executable: !pe.is_lib && pe.entry != 0,
        library: pe.is_lib,
    }))
}

fn vc_runtime(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    ["vcruntime", "msvcp", "concrt", "vcomp"]
        .iter()
        .any(|prefix| lower.starts_with(prefix))
        || lower.starts_with("msvcr") && lower != "msvcrt.dll"
}

pub fn collect(input: &Input, plan: &mut Plan, inputs: &mut Inputs) -> Result<Vec<PathBuf>> {
    let settings = &input.settings;
    let system = PathBuf::from(
        std::env::var_os("SystemRoot")
            .context("SystemRoot is required to locate Windows system DLLs")?,
    )
    .join("System32");
    let mut files = Vec::<Code>::new();
    let mut scanned = BTreeSet::new();
    let mut position = 0;
    loop {
        for (source, destination) in plan.source_files() {
            if scanned.insert(destination.clone())
                && let Some(code) = inspect(&source, &destination)?
            {
                files.push(code);
            }
        }
        if position == files.len() {
            break;
        }
        let source = files[position].source.clone();
        let destination_dir = files[position]
            .destination
            .parent()
            .context("PE output has no directory")?
            .to_path_buf();
        for name in files[position].libraries.clone() {
            let lower = name.to_ascii_lowercase();
            let vc = vc_runtime(&name);
            if vc {
                match settings.config.windows.vc_runtime.context("PE imports use the VC runtime; set bundle.windows.vc-runtime to system or app-local")? {
                    VcRuntime::System => continue,
                    VcRuntime::AppLocal => (),
                }
            } else {
                if lower.starts_with("api-ms-") || lower.starts_with("ext-ms-") {
                    continue;
                }
                let system_file = system.join(&name);
                inputs.watch(&system_file)?;
                if system_file.is_file() {
                    continue;
                }
            }
            let matches = files
                .iter()
                .filter(|code| {
                    [code.source.file_name(), code.destination.file_name()]
                        .into_iter()
                        .flatten()
                        .any(|f| f.to_str().is_some_and(|f| f.eq_ignore_ascii_case(&name)))
                })
                .collect::<Vec<_>>();
            let origins = matches
                .iter()
                .map(|code| code.source.canonicalize())
                .collect::<std::io::Result<BTreeSet<_>>>()?;
            ensure!(
                origins.len() <= 1,
                "{}: ambiguous private DLL {name}",
                source.display()
            );
            if matches.iter().any(|code| {
                code.destination.parent() == Some(&destination_dir)
                    && code
                        .destination
                        .file_name()
                        .and_then(|s| s.to_str())
                        .is_some_and(|s| s.eq_ignore_ascii_case(&name))
            }) {
                continue;
            }
            let resolved = if let Some(path) = origins.first() {
                path.clone()
            } else {
                let mut candidates = settings
                    .config
                    .library_search_paths
                    .iter()
                    .map(|p| settings.path(p).join(&name))
                    .collect::<Vec<_>>();
                // VC redistribution must be explicitly selected, never taken from an incidental SDK directory.
                if !vc {
                    candidates.push(
                        source
                            .parent()
                            .context("PE input has no directory")?
                            .join(&name),
                    );
                }
                let mut resolved = None;
                for candidate in candidates {
                    inputs.watch(&candidate)?;
                    if candidate.is_file() {
                        resolved = Some(candidate.canonicalize()?);
                        break;
                    }
                }
                resolved.with_context(|| format!("{}: missing private DLL {name}; declare files or library-search-paths for {}", source.display(), input.target.triple))?
            };
            let destination = destination_dir.join(&name);
            plan.add(&resolved, &destination, false, inputs)?;
            let code = inspect(&resolved, &destination)?
                .with_context(|| format!("{} is not a PE DLL", resolved.display()))?;
            ensure!(code.library, "{} is not a PE DLL", resolved.display());
            scanned.insert(destination);
            files.push(code);
        }
        position += 1;
    }
    let primary = PathBuf::from(format!("{}.exe", settings.name));
    ensure!(
        files
            .iter()
            .any(|file| file.destination == primary && file.executable),
        "main input is not an x86_64 PE executable"
    );
    Ok(files.into_iter().map(|code| code.destination).collect())
}
