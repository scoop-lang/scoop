use std::collections::{BTreeSet, HashMap};

use scoop_hir::{EditingReference, EditingReferenceIndex};
use scoop_identity::{ConeIdentity, SourceIdentity};
use scoop_syntax::SyntaxToken;

use super::super::extract_local::Binding;
use super::*;
use crate::{ProjectInput, ProjectSource};

type File<'a> = (&'a ProjectSource, &'a SourceInput, &'a SourceFile);
type ReferenceWorld = Vec<(ConeIdentity, EditingReferenceIndex)>;

pub(super) struct FunctionMove<'a> {
    pub from: File<'a>,
    pub to: File<'a>,
    pub name: SemanticSourceLocation,
    pub name_text: &'a str,
    pub moved: Span,
    pub imports: HashSet<Span>,
}

impl SemanticSession {
    pub(super) fn relocate_function(
        &mut self,
        snapshot: &AnalysisSnapshot,
        project: &ProjectInput,
        moved: FunctionMove<'_>,
        check: Rc<dyn Fn() -> bool>,
    ) -> Result<Option<Vec<SourceEdit>>, ProjectAnalysisError> {
        let Some(mut before) = self.relocation_indexes(snapshot, project, check.clone())? else {
            return Ok(None);
        };
        normalize_callable(&mut before, moved.from.1, &moved.name);
        let original_package = package_parts(&moved.from.2.package).collect::<Vec<_>>();
        let new_package = package_parts(&moved.to.2.package).collect::<Vec<_>>();
        let new_prefix = namespace_prefix(&new_package);
        let import = format!(
            "import {new_prefix}{}",
            scoop_parser::identifier_spelling(moved.name_text)
        );
        let mut edits = HashMap::<SourceId, Vec<SourceEdit>>::new();
        let mut add_import = HashSet::new();
        let mut redirected = HashSet::new();
        let mut found = false;
        for (cone, index) in &before {
            for (&target, _) in index
                .definitions
                .iter()
                .filter(|(_, origin)| **origin == moved.name)
            {
                found = true;
                for reference in index.references_to(target).filter(|reference| {
                    reference.origin.source.cone() == *cone
                        && !reference.is_declaration
                        && !(reference.origin.source == moved.name.source
                            && contains(moved.moved, reference.origin.span))
                }) {
                    if check() {
                        return Err(ProjectAnalysisError::Cancelled);
                    }
                    let Some((file, input, source)) =
                        source_file(snapshot, project, &reference.origin.source)?
                    else {
                        return Ok(None);
                    };
                    let mentioned = reference.origin.span;
                    if let Some((span, serves_short_name)) =
                        source.imports.iter().find_map(|import| {
                            let ImportSyntax::Exact {
                                selector, alias, ..
                            } = import
                            else {
                                return None;
                            };
                            if selector.segments().last()?.span != mentioned {
                                return None;
                            }
                            let old = original_package
                                .iter()
                                .copied()
                                .chain(std::iter::once(moved.name_text));
                            if selector.segments().map(|part| part.text.as_str()).ne(old) {
                                return None;
                            }
                            let uses_short_name = alias
                                .as_ref()
                                .is_none_or(|alias| alias.name.text == moved.name_text);
                            Some((
                                Span::new(selector.span.start, mentioned.start),
                                uses_short_name,
                            ))
                        })
                    {
                        if serves_short_name {
                            redirected.insert(file.source);
                        }
                        if original_package != new_package {
                            edits.entry(file.source).or_default().push(source_edit(
                                input,
                                span,
                                new_prefix.clone(),
                            ));
                        }
                        continue;
                    }
                    if reference.aliases.is_empty()
                        && package_parts(&source.package).ne(new_package.iter().copied())
                    {
                        add_import.insert(file.source);
                    }
                }
            }
        }
        if !found {
            return Ok(None);
        }
        for id in add_import.difference(&redirected) {
            let Some(input) = snapshot.source(*id)? else {
                return Ok(None);
            };
            let Some(ast) = input.parse().ok().and_then(|parsed| parsed.ast()) else {
                return Ok(None);
            };
            if let Some(edit) = import_insertion(input, ast, format!("{import}{}", newline(input)))
            {
                edits.entry(*id).or_default().push(edit);
            }
        }
        for changes in edits.values_mut() {
            changes.sort_unstable_by_key(|edit| (edit.span.start, edit.span.end));
            changes.dedup_by(|left, right| {
                left.span == right.span && left.replacement == right.replacement
            });
        }
        let extra = implicit_imports(
            snapshot,
            project,
            &before,
            &moved,
            &original_package,
            &new_package,
        )?;
        let Some((target_start, insertions)) = destination_edits(
            moved.from.1,
            moved.from.2,
            moved.to.1,
            moved.to.2,
            moved.moved,
            &moved.imports,
            &extra,
        ) else {
            return Ok(None);
        };
        let target_edits = edits.entry(moved.to.0.source).or_default();
        let shift: i64 = target_edits
            .iter()
            .map(|edit| edit.replacement.len() as i64 - i64::from(edit.span.end - edit.span.start))
            .sum();
        let Ok(target_start) = u32::try_from(i64::from(target_start) + shift) else {
            return Ok(None);
        };
        target_edits.extend(insertions);
        edits
            .entry(moved.from.0.source)
            .or_default()
            .push(source_edit(
                moved.from.1,
                removal_span(moved.from.1.text.as_str(), moved.moved),
                String::new(),
            ));
        for changes in edits.values_mut() {
            changes.sort_unstable_by_key(|edit| (edit.span.start, edit.span.end));
        }
        let mut changes = edits.values().flatten().cloned().collect::<Vec<_>>();
        changes.sort_unstable_by_key(|edit| (edit.source.source.0, edit.span.start, edit.span.end));
        let candidate = match snapshot.with_edits(&changes) {
            Ok(candidate) => candidate,
            Err(SnapshotEditError::Cancelled) => return Err(ProjectAnalysisError::Cancelled),
            Err(_) => return Ok(None),
        };
        let Some(mut after) =
            self.for_edits()
                .relocation_indexes(&candidate, project, check.clone())?
        else {
            return Ok(None);
        };
        let Some(target_input) = candidate.source(moved.to.0.source)? else {
            return Ok(None);
        };
        let new_origin = SemanticSourceLocation {
            source: moved.to.0.identity.clone(),
            span: Span::new(
                target_start + (moved.name.span.start - moved.moved.start),
                target_start + (moved.name.span.end - moved.moved.start),
            ),
        };
        normalize_callable(&mut after, target_input, &new_origin);
        let Some(expected) = world_bindings(
            &before,
            &moved.name,
            |location| {
                if location.source == moved.name.source && contains(moved.moved, location.span) {
                    return Some(SemanticSourceLocation {
                        source: moved.to.0.identity.clone(),
                        span: Span::new(
                            target_start + (location.span.start - moved.moved.start),
                            target_start + (location.span.end - moved.moved.start),
                        ),
                    });
                }
                let span = match project
                    .source_file(&location.source)
                    .and_then(|file| edits.get(&file.source))
                {
                    Some(edits) => updated_span(location.span, edits)?,
                    None => location.span,
                };
                Some(SemanticSourceLocation {
                    source: location.source.clone(),
                    span,
                })
            },
            &*check,
        )?
        else {
            return Ok(None);
        };
        let Some(actual) =
            world_bindings(&after, &new_origin, |origin| Some(origin.clone()), &*check)?
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
        .then_some(changes))
    }

    fn relocation_indexes(
        &mut self,
        snapshot: &AnalysisSnapshot,
        project: &ProjectInput,
        check: Rc<dyn Fn() -> bool>,
    ) -> Result<Option<ReferenceWorld>, ProjectAnalysisError> {
        let mut indexes = Vec::new();
        for (&cone, input) in &project.cones {
            if check() {
                return Err(ProjectAnalysisError::Cancelled);
            }
            if input.source().is_none() {
                continue;
            }
            let analysis = match self.project_declarations(snapshot, project, cone, check.clone()) {
                Ok(analysis) => analysis,
                Err(error) if error.is_cancelled() => return Err(ProjectAnalysisError::Cancelled),
                Err(_) => return Ok(None),
            };
            let cancellation = check.clone();
            let index = analysis
                .reference_index(move || cancellation())
                .map_err(|_| ProjectAnalysisError::Cancelled)?;
            if !index.declarations_available || !index.unavailable.is_empty() {
                return Ok(None);
            }
            indexes.push((cone, index.clone()));
        }
        Ok(Some(indexes))
    }
}

fn world_bindings(
    world: &ReferenceWorld,
    moved: &SemanticSourceLocation,
    map: impl Fn(&SemanticSourceLocation) -> Option<SemanticSourceLocation>,
    check: &dyn Fn() -> bool,
) -> Result<Option<HashSet<(SemanticSourceLocation, Binding)>>, ProjectAnalysisError> {
    let mut combined = HashSet::new();
    for (cone, index) in world {
        // Imports can introduce a new export route to the same moved callable.
        let moved_sites = index
            .definitions
            .iter()
            .filter(|(target, origin)| {
                matches!(target, EditingReferenceTarget::Callable(_)) && *origin == moved
            })
            .flat_map(|(&target, _)| {
                index
                    .references_to(target)
                    .map(|reference| &reference.origin)
            })
            .collect::<HashSet<_>>();
        let Some(bindings) = bindings(
            index,
            |reference| {
                reference.origin.source.cone() == *cone
                    && !matches!(reference.target, EditingReferenceTarget::ImportAlias(_))
                    && !(matches!(reference.target, EditingReferenceTarget::ExportBinding(_))
                        && moved_sites.contains(&reference.origin))
            },
            &map,
            check,
        )?
        else {
            return Ok(None);
        };
        combined.extend(bindings);
    }
    Ok(Some(combined))
}

fn normalize_callable(
    world: &mut ReferenceWorld,
    input: &SourceInput,
    name: &SemanticSourceLocation,
) {
    // The same function can have a name span locally and a declaration span in a dependency.
    for (_, index) in world {
        for (target, origin) in &mut index.definitions {
            if matches!(target, EditingReferenceTarget::Callable(_))
                && origin.source == name.source
                && contains(origin.span, name.span)
                && input.declaration_name_span(origin.span).ok().flatten() == Some(name.span)
            {
                origin.span = name.span;
            }
        }
    }
}

fn source_file<'a>(
    snapshot: &'a AnalysisSnapshot,
    project: &'a ProjectInput,
    source: &SourceIdentity,
) -> Result<Option<File<'a>>, ProjectAnalysisError> {
    let Some(file) = project.source_file(source) else {
        return Ok(None);
    };
    let Some(input) = snapshot.source(file.source)? else {
        return Ok(None);
    };
    Ok(input
        .parse()
        .ok()
        .and_then(|parsed| parsed.ast())
        .map(|ast| (file, input.as_ref(), ast)))
}

fn implicit_imports(
    snapshot: &AnalysisSnapshot,
    project: &ProjectInput,
    world: &ReferenceWorld,
    moved: &FunctionMove<'_>,
    original: &[&str],
    destination: &[&str],
) -> Result<Vec<String>, ProjectAnalysisError> {
    let mut imports = BTreeSet::new();
    if original == destination {
        return Ok(Vec::new());
    }
    let within = |reference: &&EditingReference| {
        reference.origin.source == moved.name.source
            && !reference.is_declaration
            && contains(moved.moved, reference.origin.span)
            && reference.aliases.is_empty()
    };
    for (cone, index) in world {
        if *cone != moved.name.source.cone() {
            continue;
        }
        for reference in index.occurrences().iter().filter(within) {
            let Some(definition) = index.definitions.get(&reference.target) else {
                continue;
            };
            if definition.source == moved.name.source && contains(moved.moved, definition.span) {
                continue;
            }
            let Some((file, input, ast)) = source_file(snapshot, project, &definition.source)?
            else {
                continue;
            };
            if file.identity.cone() != *cone
                || package_parts(&ast.package).ne(original.iter().copied())
                || previous_token(moved.from.1, reference.origin.span)
                    .is_some_and(|token| token.kind() == SyntaxKind::Dot)
            {
                continue;
            }
            if !ast.declarations.iter().any(|declaration| {
                contains(declaration.span(), definition.span)
                    && input
                        .declaration_name_span(declaration.span())
                        .ok()
                        .flatten()
                        == Some(definition.span)
            }) {
                continue;
            }
            let span = reference.origin.span;
            let name = &moved.from.1.text.as_str()[span.start as usize..span.end as usize];
            let site =
                &input.text.as_str()[definition.span.start as usize..definition.span.end as usize];
            if let Some(name) = scoop_parser::identifier_name(name)
                && scoop_parser::identifier_name(site).as_ref() == Some(&name)
            {
                imports.insert(format!(
                    "import {}{}",
                    namespace_prefix(original),
                    scoop_parser::identifier_spelling(&name)
                ));
            }
        }
    }
    Ok(imports.into_iter().collect())
}

fn namespace_prefix(parts: &[&str]) -> String {
    let mut prefix = String::new();
    for part in parts {
        prefix.push_str(&scoop_parser::identifier_spelling(part));
        prefix.push('.');
    }
    prefix
}

fn previous_token(input: &SourceInput, span: Span) -> Option<SyntaxToken> {
    let root = input.parse().ok()?.syntax();
    previous_significant(&root.token_at_offset(span.start.into()).right_biased()?)
}

fn previous_significant(token: &SyntaxToken) -> Option<SyntaxToken> {
    std::iter::successors(token.prev_token(), SyntaxToken::prev_token)
        .find(|token| token.kind() != SyntaxKind::Whitespace)
}

fn source_edit(input: &SourceInput, span: Span, replacement: String) -> SourceEdit {
    SourceEdit {
        source: input.version,
        span,
        replacement,
    }
}

fn newline(input: &SourceInput) -> &'static str {
    if input.text.as_str().contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    }
}
