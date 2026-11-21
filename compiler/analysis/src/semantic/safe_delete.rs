use scoop_ast::{Decl, DeclaredVisibility, FunctionBody, VisibilitySyntax};
use scoop_hir::{EditingReferenceTarget, SemanticSourceLocation};
use scoop_identity::RequestedConeKind;
use scoop_source::{SourceEdit, Span};

use super::*;

pub enum SafeDelete {
    Edit(SourceEdit),
    Blocked(Vec<SemanticSourceLocation>),
}

impl SemanticSession {
    pub fn safe_delete_function(
        &mut self,
        snapshot: &AnalysisSnapshot,
        source: SourceId,
        selected: Span,
        is_cancelled: impl Fn() -> bool + 'static,
    ) -> Result<Option<SafeDelete>, ProjectAnalysisError> {
        let Some(input) = snapshot.source(source)? else {
            return Ok(None);
        };
        let Some(ast) = input.parse().ok().and_then(|parsed| parsed.ast()) else {
            return Ok(None);
        };
        let Some(function) = ast.declarations.iter().find_map(|declaration| {
            let Decl::Function(function) = declaration else {
                return None;
            };
            (function.name.span.start <= selected.start
                && selected.end <= function.name.span.end
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
        }) else {
            return Ok(None);
        };
        let Some(project) = snapshot.workspace.project_for_source(source) else {
            return Ok(None);
        };
        let (cone, current) = project.source_cone(source).expect("selected source Cone");
        let file = current
            .sources
            .iter()
            .find(|file| file.source == source)
            .expect("selected source");
        let origin = SemanticSourceLocation {
            source: file.identity.clone(),
            span: function.name.span,
        };
        let kind = current.manifest.parsed().semantic().requested_kind();
        if kind == RequestedConeKind::Executable && function.name.text == "main" {
            return Ok(Some(SafeDelete::Blocked(vec![origin])));
        }
        let revision = snapshot.clone();
        let check: Rc<dyn Fn() -> bool> =
            Rc::new(move || is_cancelled() || revision.check_cancelled().is_err());
        let analysis = self
            .project_declarations(snapshot, project, cone, check.clone())
            .map_err(Clone::clone)?;
        let cancellation = check.clone();
        let index = analysis
            .reference_index(move || cancellation())
            .map_err(|_| ProjectAnalysisError::Cancelled)?;
        if !index.declarations_available
            || index
                .unavailable
                .iter()
                .any(|region| region.source == origin.source)
        {
            return Ok(None);
        }
        let targets = index.targets_at(&origin);
        let [target @ EditingReferenceTarget::Callable(_)] = targets else {
            return Ok(None);
        };
        let mut uses = Vec::new();
        for reference in index.references_to(*target) {
            if check() {
                return Err(ProjectAnalysisError::Cancelled);
            }
            if reference.origin.source != origin.source
                || !(function.span.start <= reference.origin.span.start
                    && reference.origin.span.end <= function.span.end)
            {
                uses.push(reference.origin.clone());
            }
        }
        if !uses.is_empty() {
            return Ok(Some(SafeDelete::Blocked(uses)));
        }
        let start = input
            .documentation()
            .ok()
            .and_then(|documentation| {
                documentation
                    .documents
                    .iter()
                    .find(|document| document.declaration == function.span)
            })
            .map_or(function.span.start, |document| document.span.start);
        Ok(Some(SafeDelete::Edit(SourceEdit {
            source: input.version,
            span: Span::new(start, function.span.end),
            replacement: String::new(),
        })))
    }
}
