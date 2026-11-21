use scoop_ast::ParameterSyntax;
use scoop_syntax::{SyntaxKind as K, SyntaxNode};

use super::super::extract_declaration::complete_references;
use super::super::extract_local::{
    contains,
    syntax::{argument_value_span, node_span},
};
use super::*;

#[derive(Clone)]
pub enum ParameterChange {
    AddDefault {
        name: String,
        ty: String,
        expression: String,
    },
    RemoveUnused {
        parameter: usize,
    },
    SetDefault {
        parameter: usize,
        expression: Option<String>,
    },
}

impl SemanticSession {
    pub fn change_parameters(
        &mut self,
        snapshot: &AnalysisSnapshot,
        source: SourceId,
        selection: Span,
        change: &ParameterChange,
        is_cancelled: impl Fn() -> bool + 'static,
    ) -> Result<Option<Vec<SourceEdit>>, ProjectAnalysisError> {
        let Some(input) = snapshot.source(source)? else {
            return Ok(None);
        };
        let Some(function) = function_at(input, selection) else {
            return Ok(None);
        };
        if function
            .params
            .iter()
            .any(|parameter| matches!(parameter.syntax, ParameterSyntax::Vararg { .. }))
        {
            return Ok(None);
        }
        let root = input.parse().expect("selected function parsed").syntax();
        let Some(closing) = parameter_list_end(&root, function.name.span) else {
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
        let Some(before) = complete_references(analysis, &file.identity, check.clone())? else {
            return Ok(None);
        };
        let calls = if matches!(change, ParameterChange::SetDefault { .. }) {
            Vec::new()
        } else {
            let origin = SemanticSourceLocation {
                source: file.identity.clone(),
                span: function.name.span,
            };
            let Some(calls) = checked_direct_calls(analysis, &before, &origin, check.clone())?
            else {
                return Ok(None);
            };
            if calls
                .iter()
                .any(|call| call.arguments.iter().any(|argument| argument.trailing))
            {
                return Ok(None);
            }
            calls
        };
        let Some((mut edits, parameters)) =
            parameter_edits(input, &root, function, closing, &calls, change)
        else {
            return Ok(None);
        };
        if edits.is_empty() {
            return Ok(Some(edits));
        }
        edits.sort_unstable_by_key(|edit| (edit.span.start, edit.span.end));
        let candidate = match snapshot.with_edits(&edits) {
            Ok(candidate) => candidate,
            Err(SnapshotEditError::Cancelled) => return Err(ProjectAnalysisError::Cancelled),
            Err(_) => return Ok(None),
        };
        let Some(revised) = candidate.source(source)? else {
            return Ok(None);
        };
        let Some(new_name) = updated_span(function.name.span, &edits) else {
            return Ok(None);
        };
        let Some(new_function) = function_at(revised, new_name) else {
            return Ok(None);
        };
        let revised_root = revised.parse().expect("candidate function parsed").syntax();
        let expected_closing = updated_span(Span::new(closing, closing + 1), &edits);
        if new_function
            .params
            .iter()
            .map(|parameter| &parameter.name.text)
            .ne(parameters.iter())
            || parameter_list_end(&revised_root, new_name)
                != expected_closing.map(|span| span.start)
        {
            return Ok(None);
        }
        let mut session = self.for_edits();
        let analysis = session
            .project_declarations(&candidate, project, cone, check.clone())
            .map_err(Clone::clone)?;
        let Some(after) = complete_references(analysis, &file.identity, check.clone())? else {
            return Ok(None);
        };
        let Some(expected) = bindings(
            &before,
            |reference| {
                reference.origin.source == file.identity
                    && !edits.iter().any(|edit| {
                        edit.span.start < edit.span.end
                            && contains(edit.span, reference.origin.span)
                    })
            },
            |origin| {
                Some(SemanticSourceLocation {
                    source: origin.source.clone(),
                    span: if origin.source == file.identity {
                        updated_span(origin.span, &edits)?
                    } else {
                        origin.span
                    },
                })
            },
            &*check,
        )?
        else {
            return Ok(None);
        };
        let Some(actual) = bindings(
            &after,
            |reference| reference.origin.source == file.identity,
            |origin| Some(origin.clone()),
            &*check,
        )?
        else {
            return Ok(None);
        };
        let positions = expected
            .iter()
            .map(|(origin, _)| origin)
            .collect::<HashSet<_>>();
        Ok((expected.is_subset(&actual)
            && actual
                .iter()
                .filter(|(origin, _)| positions.contains(origin))
                .all(|binding| expected.contains(binding)))
        .then_some(edits))
    }
}

fn parameter_edits(
    input: &SourceInput,
    root: &SyntaxNode,
    function: &FunctionDecl,
    closing: u32,
    calls: &[EditingAppliedCall],
    change: &ParameterChange,
) -> Option<(Vec<SourceEdit>, Vec<String>)> {
    let text = input.text.as_str();
    let mut expected = function
        .params
        .iter()
        .map(|parameter| parameter.name.text.clone())
        .collect::<Vec<_>>();
    let edit = |span, replacement| SourceEdit {
        source: input.version,
        span,
        replacement,
    };
    let edits = match change {
        ParameterChange::AddDefault {
            name,
            ty,
            expression,
        } => {
            let name = scoop_parser::identifier_name(name.trim())?;
            if expected.contains(&name) || ty.trim().is_empty() || expression.trim().is_empty() {
                return None;
            }
            let separator = if let Some(last) = function.params.last() {
                let end = node_span(&parameter_node(root, last.name.span)?).end;
                let tail = text.get(end as usize..closing as usize)?;
                if tail.trim_start().starts_with(',') {
                    " "
                } else {
                    ", "
                }
            } else {
                ""
            };
            let spelling = scoop_parser::identifier_spelling(&name);
            let parameter = format!(
                "{separator}{spelling}: {} = {}",
                ty.trim(),
                expression.trim()
            );
            expected.push(name);
            vec![edit(Span::new(closing, closing), parameter)]
        }
        ParameterChange::SetDefault {
            parameter,
            expression,
        } => {
            let parameter = function.params.get(*parameter)?;
            let existing = if let ParameterSyntax::Default { equals_span, .. } = &parameter.syntax {
                let node = parameter_node(root, parameter.name.span)?;
                let value = node
                    .children()
                    .find(|child| child.kind() == K::Expression)?;
                Some((node_span(&value), *equals_span))
            } else {
                None
            };
            match (existing, expression) {
                (Some((value, _)), Some(expression)) if !expression.trim().is_empty() => {
                    vec![edit(value, expression.trim().to_owned())]
                }
                (None, Some(expression)) if !expression.trim().is_empty() => {
                    vec![edit(
                        Span::new(parameter.span.end, parameter.span.end),
                        format!(" = {}", expression.trim()),
                    )]
                }
                (Some((value, equals)), None) => {
                    let leading =
                        text.get(parameter.ty.span.end as usize..equals.start as usize)?;
                    let start = if leading.trim().is_empty() {
                        parameter.ty.span.end
                    } else {
                        equals.start
                    };
                    vec![edit(Span::new(start, value.end), String::new())]
                }
                (None, None) => Vec::new(),
                _ => return None,
            }
        }
        ParameterChange::RemoveUnused { parameter } => {
            let removed = function.params.get(*parameter)?;
            let mut edits = Vec::new();
            for call in calls {
                if let Some(index) = call
                    .arguments
                    .iter()
                    .position(|arg| arg.parameter == *parameter)
                {
                    let argument = &call.arguments[index];
                    if !literal(root, argument_value_span(root, argument)?) {
                        return None;
                    }
                    let closing = call.span.end.checked_sub(1)?;
                    if text.as_bytes().get(closing as usize) != Some(&b')') {
                        return None;
                    }
                    let spans = call
                        .arguments
                        .iter()
                        .map(|arg| arg.span)
                        .collect::<Vec<_>>();
                    edits.push(edit(
                        remove_entry(text, &spans, index, closing)?,
                        String::new(),
                    ));
                } else if let ParameterSyntax::Default { expression, .. } = &removed.syntax
                    && !literal(root, expression.span())
                {
                    return None;
                }
            }
            expected.remove(*parameter);
            let spans = function
                .params
                .iter()
                .map(|param| parameter_node(root, param.name.span).map(|node| node_span(&node)))
                .collect::<Option<Vec<_>>>()?;
            edits.push(edit(
                remove_entry(text, &spans, *parameter, closing)?,
                String::new(),
            ));
            edits
        }
    };
    Some((edits, expected))
}

fn parameter_list_end(root: &SyntaxNode, name: Span) -> Option<u32> {
    let function = root
        .token_at_offset(name.start.into())
        .right_biased()?
        .parent_ancestors()
        .find(|node| node.kind() == K::FunctionDecl)?;
    let list = function
        .children()
        .find(|node| node.kind() == K::ParameterList)?;
    let closing = list
        .children_with_tokens()
        .filter_map(|item| item.into_token())
        .find(|token| token.kind() == K::RParen)?;
    Some(closing.text_range().start().into())
}

fn parameter_node(root: &SyntaxNode, name: Span) -> Option<SyntaxNode> {
    root.token_at_offset(name.start.into())
        .right_biased()?
        .parent_ancestors()
        .find(|node| node.kind() == K::Parameter)
}

fn literal(root: &SyntaxNode, span: Span) -> bool {
    root.token_at_offset(span.start.into())
        .right_biased()
        .is_some_and(|token| {
            u32::from(token.text_range().end()) == span.end
                && matches!(
                    token.kind(),
                    K::KwTrue | K::KwFalse | K::Integer | K::Float | K::String | K::Char
                )
        })
}

fn remove_entry(text: &str, spans: &[Span], index: usize, closing: u32) -> Option<Span> {
    let removed = *spans.get(index)?;
    let next = spans.get(index + 1);
    let gap_end = next.map_or(closing, |next| next.start);
    let gap = text.get(removed.end as usize..gap_end as usize)?;
    if gap.trim() == "," {
        let end = if let Some(next) = next {
            next.start
        } else {
            removed.end + u32::try_from(gap.find(',')? + 1).ok()?
        };
        return Some(Span::new(removed.start, end));
    }
    if gap_end != closing || !gap.trim().is_empty() {
        return None;
    }
    let Some(previous) = index
        .checked_sub(1)
        .and_then(|previous| spans.get(previous))
    else {
        return Some(removed);
    };
    let gap = text.get(previous.end as usize..removed.start as usize)?;
    if gap.trim() != "," {
        return None;
    }
    let start = previous.end + u32::try_from(gap.find(',')?).ok()?;
    Some(Span::new(start, removed.end))
}
