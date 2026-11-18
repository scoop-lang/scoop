use super::*;
use scoop_protocol::GeneratorToolInputV1;

pub fn record_generator_input(
    name: &str,
    program: &Path,
    version: u32,
) -> Result<GeneratorToolInputV1, ToolchainError> {
    let metadata = program.metadata().map_err(|source| {
        error(
            scoop_source::DiagnosticCode::E2201,
            format!("cannot inspect generator {}: {source}", program.display()),
        )
    })?;
    let registration = GeneratorRegistrationV1::new(
        name.to_owned(),
        HostPathCarrier::from_path(program)
            .map_err(|cause| error(scoop_source::DiagnosticCode::ICE2501, cause))?,
    )
    .map_err(|cause| error(scoop_source::DiagnosticCode::ICE2501, cause))?;
    Ok(GeneratorToolInputV1::new(
        registration,
        version,
        metadata.len(),
        metadata
            .modified()
            .map_err(|cause| error(scoop_source::DiagnosticCode::E2201, cause))?,
    ))
}

impl GeneratorPrograms {
    pub fn inputs_current(
        &self,
        inputs: &[GeneratorToolInputV1],
        compiler: &Path,
    ) -> Result<bool, ToolchainError> {
        for input in inputs {
            let registration = input.registration();
            let Some(program) = self.program_for_compiler(registration.name(), compiler) else {
                return Ok(false);
            };
            if program
                != registration
                    .program()
                    .to_path_buf()
                    .map_err(|cause| error(scoop_source::DiagnosticCode::E2502, cause))?
            {
                return Ok(false);
            }
            let metadata = match program.metadata() {
                Ok(metadata) => metadata,
                Err(source)
                    if matches!(
                        source.kind(),
                        std::io::ErrorKind::NotFound | std::io::ErrorKind::NotADirectory
                    ) =>
                {
                    return Ok(false);
                }
                Err(source) => {
                    return Err(error(
                        scoop_source::DiagnosticCode::E2201,
                        format!("cannot inspect generator {}: {source}", program.display()),
                    ));
                }
            };
            if metadata.len() != input.byte_length()
                || metadata
                    .modified()
                    .map_err(|cause| error(scoop_source::DiagnosticCode::E2201, cause))?
                    != input.modified()
            {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_used_bindings_and_program_updates_invalidate_results() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        let tool = root.join("tool");
        std::fs::write(&tool, "first").unwrap();
        let inputs = [record_generator_input("used", &tool, 1).unwrap()];
        let programs = |unused: &str| {
            GeneratorPrograms::from_cli(
                &["used=tool".into(), format!("unused={unused}").into()],
                root,
            )
            .unwrap()
        };
        let compiler = root.join("scoopc");
        assert!(
            programs("missing")
                .inputs_current(&inputs, &compiler)
                .unwrap()
        );
        assert!(
            programs("different-missing")
                .inputs_current(&inputs, &compiler)
                .unwrap()
        );
        assert!(
            !GeneratorPrograms::default()
                .inputs_current(&inputs, &compiler)
                .unwrap()
        );
        std::fs::write(&tool, "changed program").unwrap();
        assert!(
            !programs("missing")
                .inputs_current(&inputs, &compiler)
                .unwrap()
        );
        assert!(programs("missing").inputs_current(&[], &compiler).unwrap());
    }

    #[test]
    fn default_program_is_paired_with_the_selected_compiler() {
        let installation = tempfile::tempdir().unwrap();
        let root = installation.path().join("bin");
        std::fs::create_dir(&root).unwrap();
        let compiler = root.join(crate::host_executable_name("scoopc"));
        std::fs::write(&compiler, "compiler").unwrap();
        let selected = root
            .join("..")
            .join("bin")
            .join(compiler.file_name().unwrap());
        for (name, executable) in [
            ("scoop.codec", "scoop-codec-generator"),
            ("scoop.cli", "scoop-cli-generator"),
        ] {
            let program = root
                .canonicalize()
                .unwrap()
                .join(crate::host_executable_name(executable));
            std::fs::write(&program, "generator").unwrap();
            let input = record_generator_input(name, &program, 1).unwrap();
            assert!(
                GeneratorPrograms::default()
                    .inputs_current(&[input], &selected)
                    .unwrap()
            );
        }
    }
}
