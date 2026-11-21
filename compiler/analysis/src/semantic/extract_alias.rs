use scoop_hir::EditingValue;
use scoop_source::{SourceEdit, Span};
use scoop_syntax::SyntaxKind;

use super::extract_local::syntax::{enclosing_statement, fresh_name, node_span, trimmed_range};
use super::*;
use crate::{SnapshotEditError, SourceInput};

impl SemanticSession {
    pub fn extract_type_alias(
        &mut self,
        snapshot: &AnalysisSnapshot,
        source: SourceId,
        range: Span,
        is_cancelled: impl Fn() -> bool + 'static,
    ) -> Result<Option<SourceEdit>, ProjectAnalysisError> {
        let Some(input) = snapshot.source(source)? else {
            return Ok(None);
        };
        let Some((selection, replacement, edit)) = alias_edit(input, range) else {
            return Ok(None);
        };
        let Some(project) = snapshot.workspace.project_for_source(source) else {
            return Ok(None);
        };
        let (cone, _) = project.source_cone(source).expect("selected source Cone");
        let file = project
            .source_file_by_id(source)
            .expect("selected source file");
        let revision = snapshot.clone();
        let check: Rc<dyn Fn() -> bool> =
            Rc::new(move || is_cancelled() || revision.check_cancelled().is_err());
        let analysis = self
            .project_declarations(snapshot, project, cone, check.clone())
            .map_err(Clone::clone)?;
        let Some(original) = type_key(analysis, &file.identity, selection, check.clone())? else {
            return Ok(None);
        };
        let candidate = match snapshot.with_edits(std::slice::from_ref(&edit)) {
            Ok(candidate) => candidate,
            Err(SnapshotEditError::Cancelled) => return Err(ProjectAnalysisError::Cancelled),
            Err(_) => return Ok(None),
        };
        let mut session = self.for_edits();
        let analysis = session
            .project_declarations(&candidate, project, cone, check.clone())
            .map_err(Clone::clone)?;
        if type_key(analysis, &file.identity, replacement, check)? != Some(original) {
            return Ok(None);
        }
        Ok(Some(edit))
    }
}

fn type_key(
    analysis: &mut DeclarationAnalysis,
    source: &scoop_identity::SourceIdentity,
    span: Span,
    check: Rc<dyn Fn() -> bool>,
) -> Result<Option<scoop_identity::SignatureTypeKey>, ProjectAnalysisError> {
    let EditingValue::Known(Some(ty)) = analysis
        .source_type_application(source, span, move || check())
        .map_err(|_| ProjectAnalysisError::Cancelled)?
    else {
        return Ok(None);
    };
    Ok(analysis.resolved_type_key_for_edit(ty))
}

fn alias_edit(input: &SourceInput, range: Span) -> Option<(Span, Span, SourceEdit)> {
    let selection = trimmed_range(input, range)?;
    let root = input.parse().ok()?.syntax();
    let token = root
        .token_at_offset(selection.start.into())
        .right_biased()?;
    let selected = token
        .parent_ancestors()
        .find(|node| node.kind() == SyntaxKind::Type && node_span(node) == selection)?;
    if selected
        .descendants()
        .any(|node| node.kind() == SyntaxKind::Annotation)
    {
        return None;
    }
    let (insertion, private) = if let Some(statement) = enclosing_statement(&root, selection) {
        (node_span(&statement).start, "")
    } else {
        let top = selected.ancestors().find(|node| {
            node.kind() == SyntaxKind::Declaration
                && node
                    .parent()
                    .is_some_and(|parent| parent.kind() == SyntaxKind::SourceFile)
        })?;
        let declaration = node_span(&top);
        let start = input
            .documentation()
            .ok()
            .and_then(|documentation| {
                documentation
                    .documents
                    .iter()
                    .find(|document| document.declaration == declaration)
            })
            .map_or(declaration.start, |document| document.span.start);
        (start, "private ")
    };
    let name = fresh_name(&root, "ExtractedType")?;
    let text = input.text.as_str();
    let insertion_index = insertion as usize;
    let first = selection.start as usize;
    let last = selection.end as usize;
    let before = &text[insertion_index..first];
    let alias = &text[first..last];
    let line = text[..insertion_index].rsplit(['\r', '\n']).next()?;
    let separator = if line.chars().all(|ch| matches!(ch, ' ' | '\t')) {
        let newline = text.find(['\r', '\n']).map_or("\n", |offset| {
            if text[offset..].starts_with("\r\n") {
                "\r\n"
            } else {
                &text[offset..offset + 1]
            }
        });
        format!("{newline}{line}")
    } else {
        "; ".to_owned()
    };
    let prefix = format!("{private}typealias {name} = {alias}{separator}{before}");
    let start = u32::try_from(insertion_index + prefix.len()).ok()?;
    let end = start.checked_add(name.len() as u32)?;
    Some((
        selection,
        Span::new(start, end),
        SourceEdit {
            source: input.version,
            span: Span::new(insertion, selection.end),
            replacement: prefix + &name,
        },
    ))
}
