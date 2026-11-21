use std::collections::HashSet;

use scoop_hir::{EditingHierarchyType, EditingValue, SemanticSourceLocation};
use scoop_source::Span;

use super::*;

pub struct TypeHierarchyNode {
    pub declaration: EditingHierarchyType,
    pub context: SourceId,
}

pub struct TypeHierarchyOrigin {
    pub context: SourceId,
    pub source: SourceId,
    pub span: Span,
}

#[derive(Clone, Copy)]
pub enum TypeHierarchyDirection {
    Super,
    Sub,
}

impl SemanticSession {
    pub fn prepare_type_hierarchy(
        &mut self,
        snapshot: &AnalysisSnapshot,
        source: SourceId,
        span: Span,
        is_cancelled: impl Fn() -> bool + 'static,
    ) -> Result<Vec<TypeHierarchyNode>, ProjectAnalysisError> {
        let Some(project) = snapshot.workspace.project_for_source(source) else {
            return Ok(Vec::new());
        };
        let (cone, input) = project.source_cone(source).expect("selected source Cone");
        let file = input
            .sources
            .iter()
            .find(|file| file.source == source)
            .expect("selected source input");
        let revision = snapshot.clone();
        let check: Rc<dyn Fn() -> bool> =
            Rc::new(move || is_cancelled() || revision.check_cancelled().is_err());
        let analysis = self
            .project_declarations(snapshot, project, cone, check.clone())
            .map_err(Clone::clone)?;
        let EditingValue::Known(targets) = analysis
            .type_implementation_targets(&file.identity, span, move || check())
            .map_err(|_| ProjectAnalysisError::Cancelled)?
        else {
            return Ok(Vec::new());
        };
        Ok(targets
            .into_iter()
            .filter_map(|target| analysis.hierarchy_type(target))
            .map(|declaration| TypeHierarchyNode {
                declaration,
                context: source,
            })
            .collect())
    }

    pub fn type_hierarchy(
        &mut self,
        snapshot: &AnalysisSnapshot,
        source: SourceId,
        origin: TypeHierarchyOrigin,
        direction: TypeHierarchyDirection,
        is_cancelled: impl Fn() -> bool + 'static,
    ) -> Result<Vec<TypeHierarchyNode>, ProjectAnalysisError> {
        let Some(project) = snapshot.workspace.project_for_source(source) else {
            return Ok(Vec::new());
        };
        let Some((current, _)) = project.source_cone(origin.context) else {
            return Ok(Vec::new());
        };
        let Some(file) = project.source_file_by_id(origin.source) else {
            return Ok(Vec::new());
        };
        let location = SemanticSourceLocation {
            source: file.identity.clone(),
            span: origin.span,
        };
        let revision = snapshot.clone();
        let check: Rc<dyn Fn() -> bool> =
            Rc::new(move || is_cancelled() || revision.check_cancelled().is_err());
        let analysis = self
            .project_declarations(snapshot, project, current, check.clone())
            .map_err(Clone::clone)?;
        let Some(target) = analysis
            .hierarchy_declaration(&location, &*check)
            .map_err(|_| ProjectAnalysisError::Cancelled)?
        else {
            return Ok(Vec::new());
        };
        if matches!(direction, TypeHierarchyDirection::Super) {
            return Ok(analysis
                .hierarchy_supertypes(target)
                .into_iter()
                .filter_map(|parent| analysis.hierarchy_type(parent))
                .map(|declaration| TypeHierarchyNode {
                    declaration,
                    context: origin.context,
                })
                .collect());
        }
        let mut result = Vec::new();
        let mut seen = HashSet::new();
        for (&cone, input) in &project.cones {
            if check() {
                return Err(ProjectAnalysisError::Cancelled);
            }
            let Some(context) = input.source().and_then(|input| input.sources.first()) else {
                continue;
            };
            let analysis = match self.project_declarations(snapshot, project, cone, check.clone()) {
                Ok(analysis) => analysis,
                Err(error) if error.is_cancelled() => return Err(ProjectAnalysisError::Cancelled),
                Err(_) => continue,
            };
            let children = analysis
                .hierarchy_subtypes(target, &*check)
                .map_err(|_| ProjectAnalysisError::Cancelled)?
                .to_vec();
            result.extend(
                children
                    .into_iter()
                    .filter_map(|child| analysis.hierarchy_type(child))
                    .filter(|declaration| seen.insert(declaration.declaration))
                    .map(|declaration| TypeHierarchyNode {
                        declaration,
                        context: context.source,
                    }),
            );
        }
        Ok(result)
    }
}
