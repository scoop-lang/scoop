use std::collections::HashSet;

use scoop_ast::{Decl, FunctionBody, ImportSyntax, PackageSyntax, SourceFile};
use scoop_hir::{EditingReferenceTarget, EditingValue, SemanticSourceLocation};
use scoop_source::{SourceEdit, Span};
use scoop_syntax::SyntaxKind;

use super::extract_local::bindings;
use super::inline_local::{removal_span, updated_span};
use super::*;
use crate::{SnapshotEditError, SourceInput};

impl SemanticSession {
    pub fn move_declaration(
        &mut self,
        snapshot: &AnalysisSnapshot,
        source: SourceId,
        destination: SourceId,
        selection: Span,
        is_cancelled: impl Fn() -> bool + 'static,
    ) -> Result<Option<Vec<SourceEdit>>, ProjectAnalysisError> {
        if source == destination {
            return Ok(None);
        }
        let Some(project) = snapshot.workspace.project_for_source(source) else {
            return Ok(None);
        };
        let (cone, _) = project.source_cone(source).expect("selected source Cone");
        if project
            .source_cone(destination)
            .is_none_or(|(other, _)| other != cone)
        {
            return Ok(None);
        }
        let file = project.source_file_by_id(source).expect("selected source");
        let target = project
            .source_file_by_id(destination)
            .expect("selected destination");
        if file.role != target.role {
            return Ok(None);
        }
        let (Some(input), Some(output)) = (snapshot.source(source)?, snapshot.source(destination)?)
        else {
            return Ok(None);
        };
        let (Some(from), Some(to)) = (
            input.parse().ok().and_then(|parsed| parsed.ast()),
            output.parse().ok().and_then(|parsed| parsed.ast()),
        ) else {
            return Ok(None);
        };
        if package_parts(&from.package).ne(package_parts(&to.package)) {
            return Ok(None);
        }
        let Some(function) = from.declarations.iter().find_map(|declaration| {
            let Decl::Function(function) = declaration else {
                return None;
            };
            (contains(function.name.span, selection)
                && function.annotations.is_empty()
                && function.receiver_ty.is_none()
                && !matches!(function.body, FunctionBody::None))
            .then_some(function)
        }) else {
            return Ok(None);
        };

        let revision = snapshot.clone();
        let check: Rc<dyn Fn() -> bool> =
            Rc::new(move || is_cancelled() || revision.check_cancelled().is_err());
        let analysis = self
            .project_declarations(snapshot, project, cone, check.clone())
            .map_err(Clone::clone)?;
        let name = SemanticSourceLocation {
            source: file.identity.clone(),
            span: function.name.span,
        };
        let Some(body) = analysis
            .call_hierarchy_body(&name, check.clone())
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
        let callees = calls
            .iter()
            .filter_map(|call| call.callee)
            .collect::<Vec<_>>();
        if callees.into_iter().any(|callee| {
            analysis.is_source_location_call(callee)
                || (file.path.parent() != target.path.parent()
                    && analysis.is_relative_source_call(callee))
        }) || analysis.has_source_location_default(body.id, function.span)
        {
            return Ok(None);
        }
        let cancellation = check.clone();
        let before = analysis
            .reference_index(move || cancellation())
            .map_err(|_| ProjectAnalysisError::Cancelled)?
            .clone();
        if !before.declarations_available || !before.unavailable.is_empty() {
            return Ok(None);
        }
        let import_spans = analysis
            .file_import_uses(&file.identity)
            .filter_map(|(usage, import)| {
                (import.source == file.identity && contains(function.span, usage.span))
                    .then_some(import.span)
            })
            .collect::<HashSet<_>>();
        let moved = attached_comments(input, function.span);
        let Some((destination_start, inserts)) =
            destination_edits(input, from, output, to, moved, &import_spans)
        else {
            return Ok(None);
        };
        let delete = SourceEdit {
            source: input.version,
            span: removal_span(input.text.as_str(), moved),
            replacement: String::new(),
        };
        let edits = std::iter::once(delete).chain(inserts).collect::<Vec<_>>();
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
        if !after.declarations_available || !after.unavailable.is_empty() {
            return Ok(None);
        }
        let is_ordinary = |reference: &scoop_hir::EditingReference| {
            !matches!(reference.target, EditingReferenceTarget::ImportAlias(_))
        };
        let expected = bindings(
            &before,
            is_ordinary,
            |origin| {
                if origin.source == file.identity && contains(moved, origin.span) {
                    return Some(SemanticSourceLocation {
                        source: target.identity.clone(),
                        span: Span::new(
                            destination_start + (origin.span.start - moved.start),
                            destination_start + (origin.span.end - moved.start),
                        ),
                    });
                }
                let span = if origin.source == file.identity {
                    updated_span(origin.span, &edits[..1])?
                } else if origin.source == target.identity {
                    updated_span(origin.span, &edits[1..])?
                } else {
                    origin.span
                };
                Some(SemanticSourceLocation {
                    source: origin.source.clone(),
                    span,
                })
            },
            &*check,
        )?;
        let actual = bindings(after, is_ordinary, |origin| Some(origin.clone()), &*check)?;
        Ok(expected
            .zip(actual)
            .filter(|(expected, actual)| expected.is_subset(actual))
            .map(|_| edits))
    }
}

fn contains(outer: Span, inner: Span) -> bool {
    outer.start <= inner.start && inner.end <= outer.end
}

fn package_parts(package: &PackageSyntax) -> impl Iterator<Item = &str> {
    let path = match package {
        PackageSyntax::RootPackage => None,
        PackageSyntax::QualifiedPackage { path, .. } => Some(path),
    };
    path.into_iter()
        .flat_map(|path| path.segments())
        .map(|part| part.text.as_str())
}

fn attached_comments(input: &SourceInput, mut span: Span) -> Span {
    if let Ok(documentation) = input.documentation()
        && let Some(document) = documentation
            .documents
            .iter()
            .find(|document| document.declaration == span)
    {
        span.start = document.span.start;
    }
    let Ok(parsed) = input.parse() else {
        return span;
    };
    let root = parsed.syntax();
    let mut previous = root.token_at_offset(span.start.into()).left_biased();
    let text = input.text.as_str();
    while let Some(token) = previous {
        let start = u32::from(token.text_range().start());
        let end = u32::from(token.text_range().end());
        if end > span.start || !token.kind().is_trivia() {
            break;
        }
        if token.kind() != SyntaxKind::Whitespace {
            if token.kind() == SyntaxKind::Shebang
                || !text[..start as usize]
                    .rsplit(['\r', '\n'])
                    .next()
                    .unwrap_or_default()
                    .trim()
                    .is_empty()
                || text[end as usize..span.start as usize].lines().count() > 1
            {
                break;
            }
            span.start = start;
        }
        previous = token.prev_token();
    }
    let trailing = text[span.end as usize..]
        .split(['\r', '\n'])
        .next()
        .unwrap_or_default();
    if trailing.trim_start_matches([' ', '\t']).starts_with("//") {
        span.end += trailing.len() as u32;
    }
    span
}

fn import_spans(import: &ImportSyntax) -> (Span, u32) {
    match import {
        ImportSyntax::Exact {
            span,
            import_keyword_span,
            ..
        }
        | ImportSyntax::Star {
            span,
            import_keyword_span,
            ..
        } => (*span, import_keyword_span.start),
    }
}

fn destination_edits(
    input: &SourceInput,
    from: &SourceFile,
    output: &SourceInput,
    to: &SourceFile,
    moved: Span,
    required: &HashSet<Span>,
) -> Option<(u32, Vec<SourceEdit>)> {
    let old = input.text.as_str();
    let text = output.text.as_str();
    let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let mut imports = to
        .imports
        .iter()
        .map(|import| {
            let (span, start) = import_spans(import);
            text[start as usize..span.end as usize].trim()
        })
        .collect::<HashSet<_>>();
    let mut additions = String::new();
    for import in &from.imports {
        let (span, start) = import_spans(import);
        let syntax = old[start as usize..span.end as usize].trim();
        if required.contains(&span) && imports.insert(syntax) {
            additions.push_str(syntax);
            additions.push_str(newline);
        }
    }
    let end = u32::try_from(text.len()).ok()?;
    let mut edits = Vec::new();
    let header = to
        .imports
        .last()
        .map(|import| import_spans(import).0.end)
        .or_else(|| match &to.package {
            PackageSyntax::RootPackage => None,
            PackageSyntax::QualifiedPackage { span, .. } => Some(span.end),
        });
    let mut append = String::new();
    let mut earlier = 0;
    if !additions.is_empty() {
        let offset = header.map_or(0, |end| {
            let rest = &text[end as usize..];
            let line_break = rest.find(['\r', '\n']).unwrap_or(rest.len());
            end + line_break as u32
        });
        if offset != 0 {
            additions.insert_str(0, newline);
        }
        if offset == end {
            append.push_str(&additions);
        } else {
            earlier = additions.len();
            edits.push(SourceEdit {
                source: output.version,
                span: Span::new(offset, offset),
                replacement: additions,
            });
        }
    }
    append.push_str(newline);
    append.push_str(newline);
    let start = u32::try_from(text.len() + earlier + append.len()).ok()?;
    append.push_str(&old[moved.start as usize..moved.end as usize]);
    append.push_str(newline);
    edits.push(SourceEdit {
        source: output.version,
        span: Span::new(end, end),
        replacement: append,
    });
    Some((start, edits))
}
