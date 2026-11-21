use std::collections::HashSet;

use scoop_ast::{Decl, DeclaredVisibility, FunctionBody, FunctionDecl, VisibilitySyntax};
use scoop_hir::{
    EditingAppliedCall, EditingCallee, EditingReferenceIndex, EditingReferenceTarget, EditingValue,
    SemanticCallable, SemanticSourceLocation,
};
use scoop_source::{SourceEdit, Span};

use super::extract_local::bindings;
use super::inline_local::updated_span;
use super::*;
use crate::{SnapshotEditError, SourceInput};

impl SemanticSession {
    pub fn prepare_change_signature(
        &self,
        snapshot: &AnalysisSnapshot,
        source: SourceId,
        selection: Span,
    ) -> Result<Option<Vec<String>>, ProjectAnalysisError> {
        let Some(input) = snapshot.source(source)? else {
            return Ok(None);
        };
        Ok(function_at(input, selection).map(|function| {
            function
                .params
                .iter()
                .map(|param| param.name.text.clone())
                .collect()
        }))
    }

    pub fn change_signature(
        &mut self,
        snapshot: &AnalysisSnapshot,
        source: SourceId,
        selection: Span,
        order: &[usize],
        is_cancelled: impl Fn() -> bool + 'static,
    ) -> Result<Option<Vec<SourceEdit>>, ProjectAnalysisError> {
        let Some(input) = snapshot.source(source)? else {
            return Ok(None);
        };
        let Some(function) = function_at(input, selection) else {
            return Ok(None);
        };
        if order.len() != function.params.len()
            || order.iter().any(|&index| index >= order.len())
            || order.iter().copied().collect::<HashSet<_>>().len() != order.len()
        {
            return Ok(None);
        }
        if order.iter().copied().eq(0..order.len()) {
            return Ok(Some(Vec::new()));
        }
        let Some(project) = snapshot.workspace.project_for_source(source) else {
            return Ok(None);
        };
        let (cone, _) = project.source_cone(source).expect("selected source Cone");
        let file = project.source_file_by_id(source).expect("selected source");
        let revision = snapshot.clone();
        let check: Rc<dyn Fn() -> bool> =
            Rc::new(move || is_cancelled() || revision.check_cancelled().is_err());
        let analysis = self
            .project_declarations(snapshot, project, cone, check.clone())
            .map_err(Clone::clone)?;
        let origin = SemanticSourceLocation {
            source: file.identity.clone(),
            span: function.name.span,
        };
        let cancellation = check.clone();
        let before = analysis
            .reference_index(move || cancellation())
            .map_err(|_| ProjectAnalysisError::Cancelled)?
            .clone();
        if !before.declarations_available
            || before
                .unavailable
                .iter()
                .any(|region| region.source == file.identity)
        {
            return Ok(None);
        }
        let Some(calls) = checked_direct_calls(analysis, &before, &origin, check.clone())? else {
            return Ok(None);
        };
        let mut call_edits = Vec::new();
        for call in calls {
            let explicit = call
                .arguments
                .iter()
                .map(|argument| argument.parameter)
                .collect::<HashSet<_>>();
            let missing = (0..order.len()).filter(|index| !explicit.contains(index));
            if !missing.eq(order
                .iter()
                .copied()
                .filter(|index| !explicit.contains(index)))
            {
                return Ok(None);
            }
            for argument in &call.arguments {
                if argument.trailing || argument.parameter >= function.params.len() {
                    return Ok(None);
                }
                if argument.name_span.is_none() {
                    let name = scoop_parser::identifier_spelling(
                        &function.params[argument.parameter].name.text,
                    );
                    call_edits.push(SourceEdit {
                        source: input.version,
                        span: Span::new(argument.span.start, argument.span.start),
                        replacement: format!("{name} = "),
                    });
                }
            }
        }
        call_edits.sort_unstable_by_key(|edit| edit.span.start);
        let Some((signature, parameters)) = reorder(input, function, order, &call_edits) else {
            return Ok(None);
        };
        let mut edits = call_edits;
        edits.push(signature);
        edits.sort_unstable_by_key(|edit| edit.span.start);
        let candidate = match snapshot.with_edits(&edits) {
            Ok(candidate) => candidate,
            Err(SnapshotEditError::Cancelled) => return Err(ProjectAnalysisError::Cancelled),
            Err(_) => return Ok(None),
        };
        let mut session = self.for_edits();
        let analysis = session
            .project_declarations(&candidate, project, cone, check.clone())
            .map_err(Clone::clone)?;
        let cancellation = check.clone();
        let after = analysis
            .reference_index(move || cancellation())
            .map_err(|_| ProjectAnalysisError::Cancelled)?;
        if !after.declarations_available
            || after
                .unavailable
                .iter()
                .any(|region| region.source == file.identity)
        {
            return Ok(None);
        }
        let expected = bindings(
            &before,
            |reference| reference.origin.source == file.identity,
            |origin| {
                let span = if origin.source != file.identity {
                    origin.span
                } else if let Some(&(old, new)) = parameters
                    .iter()
                    .find(|(span, _)| inside(*span, origin.span))
                {
                    Span::new(
                        new + (origin.span.start - old.start),
                        new + (origin.span.end - old.start),
                    )
                } else {
                    updated_span(origin.span, &edits)?
                };
                Some(SemanticSourceLocation {
                    source: origin.source.clone(),
                    span,
                })
            },
            &*check,
        )?;
        let actual = bindings(
            after,
            |reference| reference.origin.source == file.identity,
            |origin| Some(origin.clone()),
            &*check,
        )?;
        Ok(expected
            .zip(actual)
            .filter(|(expected, actual)| expected.is_subset(actual))
            .map(|_| edits))
    }
}

pub(super) fn checked_direct_calls(
    analysis: &mut DeclarationAnalysis,
    references: &EditingReferenceIndex,
    origin: &SemanticSourceLocation,
    check: Rc<dyn Fn() -> bool>,
) -> Result<Option<Vec<EditingAppliedCall>>, ProjectAnalysisError> {
    let [target @ EditingReferenceTarget::Callable(_)] = references.targets_at(origin) else {
        return Ok(None);
    };
    let Some(declared) = analysis
        .functions()
        .iter()
        .find(|function| function.name_origin == *origin)
    else {
        return Ok(None);
    };
    let callee = EditingCallee::Callable(SemanticCallable::Current(declared.function));
    let mut names = HashSet::new();
    let mut direct = Vec::new();
    for body in analysis.source_bodies() {
        if body.origin.source != origin.source {
            continue;
        }
        let cancellation = check.clone();
        let EditingValue::Known(calls) = analysis
            .calls(body.id, move || cancellation())
            .map_err(|_| ProjectAnalysisError::Cancelled)?
        else {
            continue;
        };
        for call in calls.iter().filter(|call| call.callee == Some(callee)) {
            let Some(name) = call.callee_span else {
                return Ok(None);
            };
            if names.insert(name) {
                direct.push(call.clone());
            }
        }
    }
    if references.references_to(*target).any(|reference| {
        !reference.is_declaration
            && (reference.origin.source != origin.source || !names.contains(&reference.origin.span))
    }) {
        return Ok(None);
    }
    Ok(Some(direct))
}

fn function_at(input: &SourceInput, selection: Span) -> Option<&FunctionDecl> {
    input
        .parse()
        .ok()?
        .ast()?
        .declarations
        .iter()
        .find_map(|declaration| {
            let Decl::Function(function) = declaration else {
                return None;
            };
            (inside(function.name.span, selection)
                && function.params.len() > 1
                && function.receiver_ty.is_none()
                && function.annotations.is_empty()
                && !matches!(function.body, FunctionBody::None)
                && matches!(
                    function.visibility,
                    VisibilitySyntax::Explicit {
                        visibility: DeclaredVisibility::Private,
                        ..
                    }
                ))
            .then_some(function)
        })
}

fn inside(outer: Span, inner: Span) -> bool {
    outer.start <= inner.start && inner.end <= outer.end
}

fn reorder(
    input: &SourceInput,
    function: &FunctionDecl,
    order: &[usize],
    call_edits: &[SourceEdit],
) -> Option<(SourceEdit, Vec<(Span, u32)>)> {
    let text = input.text.as_str();
    let parameters = &function.params;
    let span = Span::new(parameters.first()?.span.start, parameters.last()?.span.end);
    let forward = updated_span(Span::new(span.start, span.start), call_edits)?.start;
    let mut replacement = String::new();
    let mut moved = Vec::new();
    for (slot, &index) in order.iter().enumerate() {
        let parameter = parameters[index].span;
        let new = forward.checked_add(u32::try_from(replacement.len()).ok()?)?;
        moved.push((parameter, new));
        replacement.push_str(&text[parameter.start as usize..parameter.end as usize]);
        if let Some(next) = parameters.get(slot + 1) {
            let gap = &text[parameters[slot].span.end as usize..next.span.start as usize];
            if !gap.chars().all(|ch| ch == ',' || ch.is_whitespace()) {
                return None;
            }
            replacement.push_str(gap);
        }
    }
    Some((
        SourceEdit {
            source: input.version,
            span,
            replacement,
        },
        moved,
    ))
}
