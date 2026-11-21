use std::collections::HashSet;

use scoop_ast::{ImportExposureSyntax, ImportSyntax};
use scoop_hir::SemanticSourceLocation;
use scoop_source::{SourceEdit, Span};
use scoop_syntax::SyntaxKind as K;

use super::extract_local::bindings;
use super::inline_local::{removal_span, updated_span};
use super::*;
use crate::SnapshotEditError;

impl SemanticSession {
    pub fn organize_imports(
        &mut self,
        snapshot: &AnalysisSnapshot,
        source: SourceId,
        is_cancelled: impl Fn() -> bool + 'static,
    ) -> Result<Option<Vec<SourceEdit>>, ProjectAnalysisError> {
        let Some(input) = snapshot.source(source)? else {
            return Ok(None);
        };
        let Ok(parsed) = input.parse() else {
            return Ok(None);
        };
        let Some(ast) = parsed.ast() else {
            return Ok(None);
        };
        let root = parsed.syntax();
        let file_imports = ast.imports.iter().filter_map(|import| match import {
            ImportSyntax::Exact {
                exposure: ImportExposureSyntax::Local,
                span,
                ..
            }
            | ImportSyntax::Star {
                exposure: ImportExposureSyntax::Local,
                span,
                ..
            } => Some(*span),
            _ => None,
        });
        let block_imports = root
            .descendants()
            .filter(|node| {
                node.kind() == K::ImportHeader
                    && node.ancestors().any(|ancestor| ancestor.kind() == K::Block)
            })
            .filter_map(|node| {
                let mut tokens = node
                    .descendants_with_tokens()
                    .filter_map(|element| element.into_token())
                    .filter(|token| !token.kind().is_trivia());
                let keyword = tokens.next()?;
                let last = tokens.last()?;
                (keyword.kind() == K::KwImport).then(|| {
                    Span::new(
                        keyword.text_range().start().into(),
                        last.text_range().end().into(),
                    )
                })
            });
        let imports = file_imports.chain(block_imports).collect::<Vec<_>>();
        if imports.is_empty() {
            return Ok(None);
        }
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
        let before = analysis
            .reference_index(move || cancellation())
            .map_err(|_| ProjectAnalysisError::Cancelled)?
            .clone();
        if !before.declarations_available
            || before
                .unavailable
                .iter()
                .any(|unavailable| unavailable.source == file.identity)
        {
            return Ok(None);
        }
        let used = analysis
            .file_import_uses(&file.identity)
            .map(|(_, import)| import.span)
            .collect::<HashSet<_>>();
        let mut edits = imports
            .into_iter()
            .filter(|span| !used.contains(span))
            .map(|span| SourceEdit {
                source: input.version,
                span: removal_span(input.text.as_str(), span),
                replacement: String::new(),
            })
            .collect::<Vec<_>>();
        if edits.is_empty() {
            return Ok(None);
        }
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
                .any(|unavailable| unavailable.source == file.identity)
        {
            return Ok(None);
        }
        let expected = bindings(
            &before,
            |reference| {
                reference.origin.source == file.identity
                    && !edits.iter().any(|edit| {
                        edit.span.start <= reference.origin.span.start
                            && reference.origin.span.end <= edit.span.end
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
        )?;
        let actual = bindings(
            after,
            |reference| reference.origin.source == file.identity,
            |origin| Some(origin.clone()),
            &*check,
        )?;
        Ok((expected.is_some() && expected == actual).then_some(edits))
    }
}
