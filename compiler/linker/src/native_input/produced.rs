use scoop_identity::NativeLibraryKind;
use scoop_toolchain::NativeLibraryOutput;

use super::*;

impl NativeInputs {
    pub(super) fn bind_outputs(
        &mut self,
        outputs: &[NativeLibraryOutput],
    ) -> Result<(), LinkError> {
        for output in outputs {
            let kind = match output.kind() {
                NativeLibraryKind::StaticArchive => NativeFileKind::Archive,
                NativeLibraryKind::Dynamic => NativeFileKind::Dylib,
                NativeLibraryKind::Framework => NativeFileKind::Framework,
                NativeLibraryKind::TargetDefault => {
                    unreachable!("produced libraries have an explicit kind")
                }
            };
            self.bind(&output.name, &output.binary, kind)?;
            for dependency in &output.dependencies {
                let kind = match dependency.kind() {
                    NativeLibraryKind::StaticArchive => NativeFileKind::Archive,
                    NativeLibraryKind::Dynamic => NativeFileKind::Dylib,
                    NativeLibraryKind::Framework => NativeFileKind::Framework,
                    NativeLibraryKind::TargetDefault => {
                        unreachable!("dependency has an explicit kind")
                    }
                };
                self.bind(&dependency.name, &dependency.binary, kind)?;
            }
        }
        Ok(())
    }

    fn bind(&mut self, name: &str, path: &Path, kind: NativeFileKind) -> Result<(), LinkError> {
        if let Some(previous) = self
            .bindings
            .insert(name.to_owned(), (path.to_owned(), kind))
            && previous != (path.to_owned(), kind)
        {
            return Err(error(
                scoop_source::DiagnosticCode::E2306,
                format!(
                    "conflicting explicit native outputs for {name}: {} and {}",
                    previous.0.display(),
                    path.display()
                ),
            ));
        }
        Ok(())
    }
}

pub(super) fn read(
    key: &NativeLinkRequirementKey,
    path: &Path,
    kind: NativeFileKind,
    profile: &ValidatedFinalLinkProfile,
) -> Result<locate::LibraryResolution, LinkError> {
    let compatible = matches!(
        (key.kind(), kind),
        (NativeLibraryKind::TargetDefault, _)
            | (NativeLibraryKind::StaticArchive, NativeFileKind::Archive)
            | (NativeLibraryKind::Dynamic, NativeFileKind::Dylib)
            | (NativeLibraryKind::Framework, NativeFileKind::Framework)
    );
    if !compatible {
        return Err(error(
            scoop_source::DiagnosticCode::E2306,
            format!(
                "produced native library {} is incompatible with {:?}",
                key.library().as_str(),
                key.kind()
            ),
        ));
    }
    Ok(locate::LibraryResolution {
        files: vec![locate::read(path, kind, profile, false)?],
        scripts: Vec::new(),
        system_alias: false,
    })
}
