use super::*;

impl SingleConeBuildRequest {
    pub(super) fn observe_ast(self) -> Result<SingleConeCompilation, SingleConeProductionError> {
        let current =
            load_current_input(self.current).map_err(SingleConeProductionError::Preflight)?;
        let sources = match current {
            LoadedCurrentConeInput::Manifest { manifest } => parse_manifest_current(
                &manifest,
                self.target.selection().target().id(),
                self.purpose,
            ),
            LoadedCurrentConeInput::SingleFile { input, .. } => {
                parse_single_discovered_source(&input)
            }
        }
        .map_err(SingleConeProductionError::Sources)?;
        let dumps = ast_dumps(&sources, self.emit);
        let warnings =
            CurrentConeDiagnosticSet::try_new(packages::single_file_warnings(&sources), &sources)
                .map_err(|error| {
                SingleConeProductionError::Production(Box::new(
                    CurrentConeProductionError::before_hir(CurrentConeProductionFailure::Warnings(
                        error,
                    )),
                ))
            })?;
        Ok(SingleConeCompilation::Observed(StageObservation {
            warnings,
            dumps,
        }))
    }
}

pub(super) fn ast_dumps(
    sources: &CurrentConeParsedSources,
    emit: StageDumpPolicy,
) -> Vec<EmittedStageDump> {
    capture_stage_dump(emit, StageDumpKind::Ast, || {
        sources
            .sources()
            .sources()
            .iter()
            .map(|source| scoop_ast::dump(source.ast()))
            .collect()
    })
}

pub(super) fn generated_dumps(
    sources: &[scoop_hir::GeneratedMethodSource],
    emit: StageDumpPolicy,
) -> Vec<EmittedStageDump> {
    capture_stage_dump(emit, StageDumpKind::Generated, || {
        use std::fmt::Write;

        let mut output = String::new();
        for source in sources {
            writeln!(
                output,
                "// Source: {}",
                source.identity.logical_path().as_str()
            )
            .unwrap();
            output.push_str(source.text.as_str());
            if !output.ends_with('\n') {
                output.push('\n');
            }
            output.push('\n');
        }
        output
    })
}

#[cfg(test)]
mod tests;
