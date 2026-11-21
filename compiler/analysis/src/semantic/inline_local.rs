use scoop_hir::{EditingLocalRole, EditingReferenceTarget, EditingValue, SemanticSourceLocation};
use scoop_source::{SourceEdit, Span};
use scoop_syntax::{SyntaxKind as K, SyntaxNode};

use super::extract_local::{Binding, bindings, syntax::node_span};
use super::*;
use crate::{SnapshotEditError, SourceInput};

impl SemanticSession {
    pub fn inline_local(
        &mut self,
        snapshot: &AnalysisSnapshot,
        source: SourceId,
        selected: Span,
        is_cancelled: impl Fn() -> bool + 'static,
    ) -> Result<Option<Vec<SourceEdit>>, ProjectAnalysisError> {
        let Some(input) = snapshot.source(source)? else {
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
        let cancellation = check.clone();
        let Some(body) = analysis
            .source_body_at(&file.identity, selected, move || cancellation())
            .map_err(|_| ProjectAnalysisError::Cancelled)?
        else {
            return Ok(None);
        };
        let cancellation = check.clone();
        let EditingValue::Known(facts) = analysis
            .source_body_facts(body.id, move || cancellation())
            .map_err(|_| ProjectAnalysisError::Cancelled)?
        else {
            return Ok(None);
        };
        let Some(selected) = facts
            .locals
            .iter()
            .find(|local| contains(local.span, selected))
        else {
            return Ok(None);
        };
        let definition = &selected.reference.definition;
        if definition.source != file.identity {
            return Ok(None);
        }
        let Some((statement, initializer, literal)) = initializer(input, definition.span) else {
            return Ok(None);
        };
        let source_binding =
            if literal {
                None
            } else {
                let Some(reference) = facts.locals.iter().find(|local| {
                    local.span == initializer && local.role == EditingLocalRole::Read
                }) else {
                    return Ok(None);
                };
                if !stable_local(input, reference.reference.definition.span) {
                    return Ok(None);
                }
                Some(reference.reference.definition.clone())
            };
        let binding = selected.reference.binding;
        let target = EditingReferenceTarget::Binding(binding);
        let uses = facts
            .locals
            .iter()
            .filter(|local| {
                local.reference.binding == binding && local.role != EditingLocalRole::Declaration
            })
            .collect::<Vec<_>>();
        if uses
            .iter()
            .any(|usage| usage.role != EditingLocalRole::Read)
        {
            return Ok(None);
        }
        let spans = uses.into_iter().map(|usage| usage.span).collect::<Vec<_>>();
        let mut old_types = Vec::new();
        for &span in &spans {
            let Some((ty, _)) = analysis.expression_type_for_edit(body.id, span) else {
                return Ok(None);
            };
            old_types.push(ty);
        }
        let Some(before) = analysis
            .body_reference_index(&body, check.clone())
            .map_err(|_| ProjectAnalysisError::Cancelled)?
        else {
            return Ok(None);
        };
        let removed = removal_span(input.text.as_str(), statement);
        let value = &input.text.as_str()[initializer.start as usize..initializer.end as usize];
        let mut edits = vec![SourceEdit {
            source: input.version,
            span: removed,
            replacement: String::new(),
        }];
        edits.extend(spans.iter().map(|&span| SourceEdit {
            source: input.version,
            span,
            replacement: format!("({value})"),
        }));
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
        let entry = Span::new(body.origin.span.start, body.origin.span.start);
        let cancellation = check.clone();
        let Some(body) = analysis
            .source_body_at(&file.identity, entry, move || cancellation())
            .map_err(|_| ProjectAnalysisError::Cancelled)?
        else {
            return Ok(None);
        };
        let Some(after) = analysis
            .body_reference_index(&body, check.clone())
            .map_err(|_| ProjectAnalysisError::Cancelled)?
        else {
            return Ok(None);
        };
        let map_origin = |origin: &SemanticSourceLocation| {
            Some(SemanticSourceLocation {
                source: origin.source.clone(),
                span: if origin.source == file.identity {
                    updated_span(origin.span, &edits)?
                } else {
                    origin.span
                },
            })
        };
        let Some(expected) = bindings(
            &before,
            |reference| {
                reference.target != target
                    && (reference.origin.source != file.identity
                        || !contains(removed, reference.origin.span))
            },
            map_origin,
            &*check,
        )?
        else {
            return Ok(None);
        };
        let Some(actual) = bindings(&after, |_| true, |origin| Some(origin.clone()), &*check)?
        else {
            return Ok(None);
        };
        if !expected.is_subset(&actual) {
            return Ok(None);
        }
        for (span, old) in spans.into_iter().zip(old_types) {
            let Some(new) = updated_span(span, &edits) else {
                return Ok(None);
            };
            let inside = Span::new(new.start + 1, new.end - 1);
            if ![new, inside].into_iter().any(|span| {
                analysis
                    .expression_type_for_edit(body.id, span)
                    .is_some_and(|(ty, _)| ty == old)
            }) {
                return Ok(None);
            }
            if let Some(binding) = &source_binding {
                let Some(binding) = map_origin(binding) else {
                    return Ok(None);
                };
                let origin = SemanticSourceLocation {
                    source: file.identity.clone(),
                    span: inside,
                };
                if !actual.contains(&(origin, Binding::Source(binding))) {
                    return Ok(None);
                }
            }
        }
        Ok(Some(edits))
    }
}

fn contains(outer: Span, inner: Span) -> bool {
    outer.start <= inner.start && inner.end <= outer.end
}

fn binding_statement(root: &SyntaxNode, span: Span) -> Option<SyntaxNode> {
    root.token_at_offset(span.start.into())
        .right_biased()?
        .parent_ancestors()
        .find(|node| {
            node.kind() == K::Statement
                && node
                    .children_with_tokens()
                    .find(|item| !item.kind().is_trivia())
                    .is_some_and(|item| item.kind() == K::KwVal)
                && node
                    .children_with_tokens()
                    .any(|item| item.kind() == K::Equal)
        })
}

fn stable_local(input: &SourceInput, span: Span) -> bool {
    let Some(root) = input.parse().ok().map(|parsed| parsed.syntax()) else {
        return false;
    };
    root.token_at_offset(span.start.into())
        .right_biased()
        .is_some_and(|token| {
            token.parent_ancestors().any(|node| {
                node.kind() == K::Parameter
                    && !node
                        .children_with_tokens()
                        .any(|part| part.kind() == K::KwVar)
            })
        })
        || binding_statement(&root, span).is_some_and(|statement| {
            statement
                .children()
                .find(|node| node.kind() == K::Pattern)
                .is_some_and(|pattern| contains(node_span(&pattern), span))
        })
}

fn initializer(input: &SourceInput, span: Span) -> Option<(Span, Span, bool)> {
    let root = input.parse().ok()?.syntax();
    let statement = binding_statement(&root, span)?;
    let pattern = statement
        .children()
        .find(|node| node.kind() == K::Pattern)?;
    if node_span(&pattern) != span {
        return None;
    }
    let value = node_span(
        &statement
            .children()
            .find(|node| node.kind() == K::Expression)?,
    );
    let token = root.token_at_offset(value.start.into()).right_biased()?;
    if u32::from(token.text_range().end()) != value.end {
        return None;
    }
    let literal = match token.kind() {
        K::KwTrue | K::KwFalse | K::Integer | K::Float | K::Char | K::String => true,
        K::Ident | K::EscapedIdent => false,
        _ => return None,
    };
    Some((node_span(&statement), value, literal))
}

fn removal_span(text: &str, statement: Span) -> Span {
    let begin = statement.start as usize;
    let trailing = &text[statement.end as usize..];
    let rest = trailing.trim_start_matches([' ', '\t']);
    if let Some(after) = rest.strip_prefix(';') {
        let after = after.trim_start_matches([' ', '\t']);
        return Span::new(
            statement.start,
            statement.end + (trailing.len() - after.len()) as u32,
        );
    }
    let line = text[..begin]
        .rsplit(['\r', '\n'])
        .next()
        .unwrap_or_default();
    if line.chars().all(|char| matches!(char, ' ' | '\t'))
        && let Some(after) = rest
            .strip_prefix("\r\n")
            .or_else(|| rest.strip_prefix(['\r', '\n']))
    {
        return Span::new(
            statement.start - line.len() as u32,
            statement.end + (trailing.len() - after.len()) as u32,
        );
    }
    statement
}

fn updated_span(span: Span, edits: &[SourceEdit]) -> Option<Span> {
    let offset = |position| {
        let mut shift = 0i64;
        for edit in edits {
            if position <= edit.span.start {
                break;
            }
            if position < edit.span.end {
                return None;
            }
            shift += edit.replacement.len() as i64 - i64::from(edit.span.end - edit.span.start);
        }
        u32::try_from(i64::from(position) + shift).ok()
    };
    Some(Span::new(offset(span.start)?, offset(span.end)?))
}
