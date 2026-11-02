use super::*;

const GNU_ELF_OPTIONS: &[&str] = &[
    "-Wl,--eh-frame-hdr",
    "-Wl,--enable-new-dtags",
    "-Wl,--build-id=sha1",
    "-Wl,-z,relro,-z,now,-z,noexecstack,-z,text",
    "-Wl,--no-undefined",
];

/// GNU-linker mechanics for an already selected ELF output and link mode.
///
/// Platform policy, tool discovery, CRT inputs and system libraries remain in
/// the concrete OS final-link profile.
pub(super) struct GnuElfLinkPlan<'a> {
    output: &'a OutputConfiguration,
    mode: LinkMode,
}

impl<'a> GnuElfLinkPlan<'a> {
    pub(super) const fn new(output: &'a OutputConfiguration, mode: LinkMode) -> Self {
        Self { output, mode }
    }

    pub(super) fn linker_args(&self) -> Vec<&'static str> {
        GNU_ELF_OPTIONS
            .iter()
            .copied()
            .chain(self.output_flags().iter().copied())
            .collect()
    }

    pub(super) fn metadata_script(&self) -> &'static str {
        match self.mode {
            LinkMode::Static => include_str!("../elf/static.ld"),
            LinkMode::Dynamic => include_str!("../elf/dynamic.ld"),
        }
    }

    pub(super) fn configure_command(
        &self,
        command: &mut Command,
        scratch: &Path,
        output: &Path,
    ) -> Result<(), ToolchainError> {
        std::fs::create_dir_all(scratch).map_err(error)?;
        let script = scratch.join("scoop-metadata.ld");
        std::fs::write(&script, self.metadata_script()).map_err(error)?;
        command
            .args(GNU_ELF_OPTIONS)
            .args(self.output_flags())
            .arg("-o")
            .arg(output)
            .args(["-Xlinker", "-T", "-Xlinker"])
            .arg(script);
        if self.output.kind() == FinalOutputKind::SharedLibrary {
            let exports = scratch.join("native-app.exports");
            std::fs::write(
                &exports,
                format!(
                    "{{ global: {}; local: *; }};\n",
                    super::output::NATIVE_APP_ENTRY
                ),
            )
            .map_err(error)?;
            command
                .args(["-Xlinker", "--version-script", "-Xlinker"])
                .arg(exports)
                .args(["-Xlinker", "-soname", "-Xlinker"])
                .arg(self.output.loader_name(output, "")?);
        }
        Ok(())
    }

    fn output_flags(&self) -> &'static [&'static str] {
        if self.output.kind() == FinalOutputKind::SharedLibrary {
            return &["-shared"];
        }
        match self.mode {
            LinkMode::Static => &["-static", "-no-pie"],
            LinkMode::Dynamic => &["-pie"],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gnu_elf_plan_uses_only_preselected_output_and_mode() {
        let output = OutputConfiguration::SharedLibrary {
            loader_name: Some("libscoop-example.so.1".to_owned()),
        };
        let plan = GnuElfLinkPlan::new(&output, LinkMode::Dynamic);
        let arguments = plan.linker_args();
        assert!(arguments.contains(&"-shared"));
        assert!(!arguments.contains(&"-pthread"));
        assert!(!arguments.iter().any(|argument| argument.contains("-lc")));

        let scratch = tempfile::tempdir().unwrap();
        let executable = scratch.path().join("libscoop-example.so");
        let mut command = Command::new("cc");
        plan.configure_command(&mut command, scratch.path(), &executable)
            .unwrap();
        let command = command
            .get_args()
            .map(|argument| argument.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        assert!(
            command
                .windows(2)
                .any(|arguments| { arguments == ["-Xlinker", "libscoop-example.so.1"] })
        );
        assert_eq!(
            std::fs::read_to_string(scratch.path().join("scoop-metadata.ld")).unwrap(),
            include_str!("../elf/dynamic.ld")
        );
    }
}
