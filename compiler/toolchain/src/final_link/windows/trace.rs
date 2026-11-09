//! Record implicit linker inputs once, excluding already-accounted-for objects.
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

pub(super) fn inputs(
    output: &std::process::Output,
    scratch: &Path,
    explicit_inputs: &[PathBuf],
) -> Vec<PathBuf> {
    let scratch = dunce::simplified(scratch);
    let explicit: BTreeSet<_> = explicit_inputs
        .iter()
        .map(|path| dunce::simplified(path))
        .collect();
    let mut paths = BTreeSet::new();
    for bytes in [&output.stdout, &output.stderr] {
        for line in String::from_utf8_lossy(bytes).lines() {
            let text = line
                .trim()
                .split_once(": Reading ")
                .map(|(_, path)| path)
                .or_else(|| line.trim().strip_prefix("Searching "))
                .unwrap_or(line.trim())
                .trim_end_matches(':');
            let text = text
                .split_once(".a(")
                .map(|(path, _)| format!("{path}.a"))
                .or_else(|| {
                    text.split_once(".lib(")
                        .map(|(path, _)| format!("{path}.lib"))
                })
                .unwrap_or_else(|| text.to_owned());
            let path = dunce::simplified(Path::new(&text));
            if path.is_absolute() && !path.starts_with(scratch) && !explicit.contains(path) {
                paths.insert(path.to_owned());
            }
        }
    }
    paths.into_iter().filter(|path| path.is_file()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trace_keeps_implicit_libraries_but_excludes_explicit_objects_and_scratch() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().canonicalize().unwrap();
        let scratch = root.join("scratch");
        std::fs::create_dir(&scratch).unwrap();
        let object = root.join("cached object.o");
        let batch = root.join("selected members.lib");
        let library = root.join("CRT library.lib");
        let implicit_object = root.join("CRT startup.obj");
        let temporary = scratch.join("startup.obj");
        for path in [&object, &batch, &library, &implicit_object, &temporary] {
            std::fs::write(path, b"input").unwrap();
        }
        let paths = [&object, &batch, &library, &implicit_object, &temporary];
        let trace = paths
            .into_iter()
            .flat_map(|path| {
                let path = std::fs::canonicalize(path).unwrap();
                [
                    format!("lld-link: Reading {}\n", path.display()),
                    format!("lld-link: Reading {}\n", path.display()),
                ]
            })
            .chain([format!("Searching {}:\n", library.display())])
            .collect::<String>();
        let output = std::process::Output {
            status: Default::default(),
            stdout: trace.into_bytes(),
            stderr: format!("lld-link: Reading {}(member.obj)\n", library.display()).into_bytes(),
        };
        let actual = inputs(&output, &scratch, &[object, batch]);
        let expected: BTreeSet<_> = [library, implicit_object]
            .iter()
            .map(|path| dunce::simplified(&path.canonicalize().unwrap()).to_owned())
            .collect();
        assert_eq!(actual, expected.into_iter().collect::<Vec<_>>());
    }
}
