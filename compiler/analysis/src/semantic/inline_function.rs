use scoop_ast::{
    Decl, DeclaredVisibility, FunctionBody, FunctionDecl, ParameterSyntax, StatementKind,
    VisibilitySyntax,
};
use scoop_hir::{EditingCallee, EditingValue, SemanticCallable, SemanticSourceLocation};
use scoop_source::{SourceEdit, Span};
use scoop_syntax::SyntaxKind as K;

use super::extract_declaration::complete_references;
use super::extract_local::{bindings, contains, syntax::trimmed_range};
use super::inline_local::updated_span;
use super::*;
use crate::SnapshotEditError;

mod edit;

impl SemanticSession {
    pub fn inline_function(
        &mut self,
        snapshot: &AnalysisSnapshot,
        source: SourceId,
        selection: Span,
        is_cancelled: impl Fn() -> bool + 'static,
    ) -> Result<Option<SourceEdit>, ProjectAnalysisError> {
        let Some(input) = snapshot.source(source)? else {
            return Ok(None);
        };
        let selection = if selection.start == selection.end {
            selection
        } else if let Some(selection) = trimmed_range(input, selection) {
            selection
        } else {
            return Ok(None);
        };
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
        let cancellation = check.clone();
        let Some(body) = analysis
            .source_body_at(&file.identity, selection, move || cancellation())
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
        let Some(call) = calls
            .iter()
            .find(|call| {
                (call.span == selection
                    || call
                        .callee_span
                        .is_some_and(|name| contains(name, selection)))
                    && matches!(
                        call.callee,
                        Some(EditingCallee::Callable(SemanticCallable::Current(_)))
                    )
            })
            .cloned()
        else {
            return Ok(None);
        };
        let Some(EditingCallee::Callable(SemanticCallable::Current(function))) = call.callee else {
            return Ok(None);
        };
        let Some(declared) = analysis
            .functions()
            .iter()
            .find(|item| item.function == function)
        else {
            return Ok(None);
        };
        if declared.name_origin.source != file.identity
            || call.arguments.iter().any(|argument| argument.trailing)
        {
            return Ok(None);
        }
        let Ok(parsed) = input.parse() else {
            return Ok(None);
        };
        let Some(function) = parsed.ast().and_then(|ast| {
            ast.declarations.iter().find_map(|declaration| {
                let Decl::Function(function) = declaration else {
                    return None;
                };
                (function.name.span == declared.name_origin.span).then_some(function)
            })
        }) else {
            return Ok(None);
        };
        if contains(function.span, call.span) || !ordinary_private(function) {
            return Ok(None);
        }
        let Some((statements, expression)) = inline_body(function) else {
            return Ok(None);
        };
        let root = parsed.syntax();
        if root
            .descendants_with_tokens()
            .filter_map(|element| element.into_token())
            .any(|token| {
                token.kind() == K::KwReturn
                    && [statements, Some(expression)]
                        .into_iter()
                        .flatten()
                        .any(|span| {
                            contains(
                                span,
                                Span::new(
                                    token.text_range().start().into(),
                                    token.text_range().end().into(),
                                ),
                            )
                        })
            })
        {
            return Ok(None);
        }
        let Some((original_type, _)) = analysis.expression_type_for_edit(body.id, call.span) else {
            return Ok(None);
        };
        let mut spelling = |ty| match analysis.describe_call_type(&call.context, ty).ok()? {
            EditingValue::Known(description) => Some(description.name),
            EditingValue::Unavailable(_) => None,
        };
        let Some(parameter_types) = call
            .parameters
            .iter()
            .map(|parameter| spelling(parameter.value_type))
            .collect::<Option<Vec<_>>>()
        else {
            return Ok(None);
        };
        let Some(result_type) = spelling(call.result) else {
            return Ok(None);
        };
        let Some(before) = complete_references(analysis, &file.identity, check.clone())? else {
            return Ok(None);
        };
        let Some(edit) = edit::InlineEdit::build(edit::InlineCall {
            input,
            root: &root,
            source: &file.identity,
            references: &before,
            call: &call,
            function,
            parameter_types,
            result_type: &result_type,
            statements,
            expression,
        }) else {
            return Ok(None);
        };
        for span in std::iter::once(call.span).chain(edit.copied_spans()) {
            if !source_can_move(analysis, &file.identity, span, check.clone())? {
                return Ok(None);
            }
        }
        let edits = std::slice::from_ref(&edit.source_edit);
        let candidate = match snapshot.with_edits(edits) {
            Ok(candidate) => candidate,
            Err(SnapshotEditError::Cancelled) => return Err(ProjectAnalysisError::Cancelled),
            Err(_) => return Ok(None),
        };
        let mut session = self.for_edits();
        let analysis = session
            .project_declarations(&candidate, project, cone, check.clone())
            .map_err(Clone::clone)?;
        let new = edit.replacement();
        let inner = Span::new(new.start + 1, new.end - 1);
        let cancellation = check.clone();
        let Some(body) = analysis
            .source_body_at(&file.identity, inner, move || cancellation())
            .map_err(|_| ProjectAnalysisError::Cancelled)?
        else {
            return Ok(None);
        };
        if ![new, inner].into_iter().any(|span| {
            analysis
                .expression_type_for_edit(body.id, span)
                .is_some_and(|(ty, _)| ty == original_type)
        }) {
            return Ok(None);
        }
        let Some(after) = complete_references(analysis, &file.identity, check.clone())? else {
            return Ok(None);
        };
        let map_unchanged = |origin: &SemanticSourceLocation| {
            Some(SemanticSourceLocation {
                source: origin.source.clone(),
                span: if origin.source == file.identity {
                    updated_span(origin.span, edits)?
                } else {
                    origin.span
                },
            })
        };
        let Some(mut expected) = bindings(
            &before,
            |reference| {
                reference.origin.source != file.identity
                    || !contains(call.span, reference.origin.span)
            },
            map_unchanged,
            &*check,
        )?
        else {
            return Ok(None);
        };
        let Some(copied) = bindings(
            &before,
            |reference| {
                reference.origin.source == file.identity && edit.copies(reference.origin.span)
            },
            |origin| {
                if origin.source == file.identity
                    && let Some(span) = edit.copied_position(origin.span)
                {
                    Some(SemanticSourceLocation {
                        source: origin.source.clone(),
                        span,
                    })
                } else {
                    map_unchanged(origin)
                }
            },
            &*check,
        )?
        else {
            return Ok(None);
        };
        expected.extend(copied);
        let Some(actual) = bindings(&after, |_| true, |origin| Some(origin.clone()), &*check)?
        else {
            return Ok(None);
        };
        let positions = expected
            .iter()
            .map(|(origin, _)| origin)
            .collect::<std::collections::HashSet<_>>();
        Ok((expected.is_subset(&actual)
            && actual
                .iter()
                .filter(|(origin, _)| positions.contains(origin))
                .all(|binding| expected.contains(binding)))
        .then_some(edit.source_edit))
    }
}

fn ordinary_private(function: &FunctionDecl) -> bool {
    matches!(
        function.visibility,
        VisibilitySyntax::Explicit {
            visibility: DeclaredVisibility::Private,
            ..
        }
    ) && function.annotations.is_empty()
        && !function.is_suspend
        && function.context_parameters.is_empty()
        && function.receiver_ty.is_none()
        && function.type_params.is_empty()
        && function
            .params
            .iter()
            .all(|parameter| !matches!(parameter.syntax, ParameterSyntax::Vararg { .. }))
}

fn inline_body(function: &FunctionDecl) -> Option<(Option<Span>, Span)> {
    match &function.body {
        FunctionBody::Expr(expression) => Some((None, expression.span())),
        FunctionBody::Block(block) => {
            let (last, earlier) = block.statements.split_last()?;
            let StatementKind::Return {
                label: None,
                value: Some(value),
            } = &last.kind
            else {
                return None;
            };
            let statements = earlier
                .first()
                .zip(earlier.last())
                .map(|(first, last)| Span::new(first.span.start, last.span.end));
            Some((statements, value.span()))
        }
        FunctionBody::None => None,
    }
}

pub(super) fn source_can_move(
    analysis: &mut DeclarationAnalysis,
    source: &scoop_identity::SourceIdentity,
    span: Span,
    check: Rc<dyn Fn() -> bool>,
) -> Result<bool, ProjectAnalysisError> {
    let cancellation = check.clone();
    let Some(body) = analysis
        .source_body_at(source, span, move || cancellation())
        .map_err(|_| ProjectAnalysisError::Cancelled)?
    else {
        return Ok(false);
    };
    let EditingValue::Known(calls) = analysis
        .calls(body.id, move || check())
        .map_err(|_| ProjectAnalysisError::Cancelled)?
    else {
        return Ok(false);
    };
    let callees = calls
        .iter()
        .filter(|call| contains(span, call.span))
        .filter_map(|call| call.callee)
        .collect::<Vec<_>>();
    Ok(!analysis.has_source_location_default(body.id, span)
        && callees
            .into_iter()
            .all(|callee| !analysis.is_source_location_call(callee)))
}
