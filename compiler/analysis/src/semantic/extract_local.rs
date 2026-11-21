use std::collections::HashSet;

use scoop_hir::{
    EditingReferenceIndex, EditingReferenceTarget, EditingValue, SemanticSourceLocation,
};
use scoop_source::{SourceEdit, Span};

use super::{ProjectAnalysisError, SemanticSession};
use crate::{AnalysisSnapshot, SnapshotEditError, SourceId};

mod syntax;
use syntax::Selection;

impl SemanticSession {
    pub fn extract_local(
        &mut self,
        snapshot: &AnalysisSnapshot,
        source_id: SourceId,
        range: Span,
        is_cancelled: impl Fn() -> bool + 'static,
    ) -> Result<Option<SourceEdit>, ProjectAnalysisError> {
        snapshot.check_cancelled()?;
        let Some(source) = snapshot.source(source_id)? else {
            return Ok(None);
        };
        let Some(selection) = Selection::find(source, range) else {
            return Ok(None);
        };
        let Some(project) = snapshot.workspace.project_for_source(source_id) else {
            return Ok(None);
        };
        let (current, cone) = project
            .source_cone(source_id)
            .expect("selected source Cone");
        let file = cone
            .sources
            .iter()
            .find(|file| file.source == source_id)
            .expect("selected source input");
        let snapshot_check = snapshot.clone();
        let check: std::rc::Rc<dyn Fn() -> bool> =
            std::rc::Rc::new(move || is_cancelled() || snapshot_check.check_cancelled().is_err());

        let analysis = self
            .project_declarations(snapshot, project, current, check.clone())
            .map_err(Clone::clone)?;
        let cancellation = check.clone();
        let Some(body) = analysis
            .source_body_at(&file.identity, selection.expression, move || cancellation())
            .map_err(|_| ProjectAnalysisError::Cancelled)?
        else {
            return Ok(None);
        };
        let cancellation = check.clone();
        let EditingValue::Known(calls) = analysis
            .calls(body.id, move || cancellation())
            .map_err(|_| ProjectAnalysisError::Cancelled)?
        else {
            return Ok(None);
        };
        if selection.call.is_some_and(|span| {
            !calls
                .iter()
                .any(|call| call.span == span && call.callee.is_some())
        }) {
            return Ok(None);
        }
        let moved_callees = calls
            .iter()
            .filter(|call| contains(selection.expression, call.span))
            .filter_map(|call| call.callee)
            .collect::<Vec<_>>();
        if moved_callees
            .into_iter()
            .any(|callee| analysis.is_source_location_call(callee))
        {
            return Ok(None);
        }
        let Some((value_type, annotation)) =
            analysis.expression_type_for_edit(body.id, selection.expression)
        else {
            return Ok(None);
        };
        let Some(before) = analysis
            .body_reference_index(&body, check.clone())
            .map_err(|_| ProjectAnalysisError::Cancelled)?
        else {
            return Ok(None);
        };
        let Some(edit) = selection.edit(source, annotation.as_deref()) else {
            return Ok(None);
        };
        let candidate = match snapshot.with_edits(std::slice::from_ref(&edit.source_edit)) {
            Ok(candidate) => candidate,
            Err(SnapshotEditError::Cancelled) => return Err(ProjectAnalysisError::Cancelled),
            Err(_) => return Ok(None),
        };

        let mut session = self.for_edits();
        let analysis = session
            .project_declarations(&candidate, project, current, check.clone())
            .map_err(Clone::clone)?;
        let cancellation = check.clone();
        let Some(body) = analysis
            .source_body_at(&file.identity, edit.replacement, move || cancellation())
            .map_err(|_| ProjectAnalysisError::Cancelled)?
        else {
            return Ok(None);
        };
        for span in [edit.initializer, edit.replacement] {
            if analysis
                .expression_type_for_edit(body.id, span)
                .is_none_or(|(ty, _)| ty != value_type)
            {
                return Ok(None);
            }
        }
        let Some(after) = analysis
            .body_reference_index(&body, check.clone())
            .map_err(|_| ProjectAnalysisError::Cancelled)?
        else {
            return Ok(None);
        };
        if !preserves_bindings(&before, &after, &file.identity, &edit, &*check)? {
            return Ok(None);
        }
        Ok(Some(edit.source_edit))
    }
}

fn contains(outer: Span, inner: Span) -> bool {
    outer.start <= inner.start && inner.end <= outer.end
}

#[derive(Clone, PartialEq, Eq, Hash)]
enum Binding {
    Source(SemanticSourceLocation),
    Declaration(EditingReferenceTarget),
}

fn bindings(
    index: &EditingReferenceIndex,
    mut map: impl FnMut(&SemanticSourceLocation) -> Option<SemanticSourceLocation>,
    is_cancelled: &dyn Fn() -> bool,
) -> Result<Option<HashSet<(SemanticSourceLocation, Binding)>>, ProjectAnalysisError> {
    let mut bindings = HashSet::new();
    for reference in index.occurrences() {
        if is_cancelled() {
            return Err(ProjectAnalysisError::Cancelled);
        }
        let target = match index.definitions.get(&reference.target) {
            Some(definition) => match map(definition) {
                Some(definition) => Binding::Source(definition),
                None => return Ok(None),
            },
            None if reference.target.is_local() => return Ok(None),
            None => Binding::Declaration(reference.target),
        };
        let Some(origin) = map(&reference.origin) else {
            return Ok(None);
        };
        bindings.insert((origin, target));
    }
    Ok(Some(bindings))
}

fn preserves_bindings(
    before: &EditingReferenceIndex,
    after: &EditingReferenceIndex,
    source: &scoop_identity::SourceIdentity,
    edit: &syntax::Extraction,
    is_cancelled: &dyn Fn() -> bool,
) -> Result<bool, ProjectAnalysisError> {
    let Some(expected) = bindings(
        before,
        |origin| {
            let span = if &origin.source == source {
                edit.map(origin.span)?
            } else {
                origin.span
            };
            Some(SemanticSourceLocation {
                source: origin.source.clone(),
                span,
            })
        },
        is_cancelled,
    )?
    else {
        return Ok(false);
    };
    let Some(actual) = bindings(after, |origin| Some(origin.clone()), is_cancelled)? else {
        return Ok(false);
    };
    let positions = expected
        .iter()
        .map(|(origin, _)| origin)
        .collect::<HashSet<_>>();
    Ok(actual
        .iter()
        .filter(|(origin, _)| positions.contains(origin))
        .count()
        == expected.len()
        && expected.is_subset(&actual))
}
