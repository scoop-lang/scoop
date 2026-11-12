use super::*;

pub(super) struct ImportRepair {
    pub path: Vec<String>,
    pub diagnostic: Span,
    edit: SourceEdit,
}

pub(super) fn repairs(
    context: &Context<'_>,
    semantics: &mut SemanticSession,
    range: Span,
) -> Result<Vec<ImportRepair>, ResponseError> {
    let diagnostics = crate::diagnostics::semantic(context.snapshot, context.source_id, semantics)?;
    let mut paths = std::collections::BTreeMap::new();
    for diagnostic in diagnostics {
        context
            .cancellation
            .check()
            .map_err(|_| super::super::semantic::cancelled())?;
        let Some(span) = diagnostic.span else {
            continue;
        };
        if range.start > span.end || range.end < span.start {
            continue;
        }
        for note in diagnostic.notes {
            if !note.import_path.is_empty() {
                paths.entry(note.import_path).or_insert(span);
            }
        }
    }
    let mut repairs = Vec::with_capacity(paths.len());
    for (path, diagnostic) in paths {
        if let Some(edit) = context
            .source
            .import_fix(&path)
            .map_err(|error| convert::error(ErrorCode::RequestFailed, error))?
        {
            repairs.push(ImportRepair {
                path,
                diagnostic,
                edit,
            });
        }
    }
    Ok(repairs)
}

pub(super) fn action(
    context: &Context<'_>,
    repair: ImportRepair,
) -> Result<CodeAction, ResponseError> {
    let title = format!(
        "Import {}",
        repair
            .path
            .iter()
            .map(|part| scoop_analysis::identifier_spelling(part))
            .collect::<Vec<_>>()
            .join(".")
    );
    super::action(
        context,
        title,
        repair.edit,
        Repair::Import {
            path: repair.path,
            start: repair.diagnostic.start,
            end: repair.diagnostic.end,
            analysis_revision: context.snapshot.revision.0,
        },
    )
}
