use std::collections::HashMap;

use scoop_hir::{EditingCallHierarchyDeclaration as Node, SemanticSourceLocation};
use scoop_source::Span;

use super::*;
use crate::ProjectInput;

#[derive(Clone, Copy)]
pub enum CallHierarchyDirection {
    Incoming,
    Outgoing,
}

pub struct CallHierarchyRelation {
    pub declaration: Node,
    pub sites: Vec<SemanticSourceLocation>,
}

impl SemanticSession {
    pub fn prepare_call_hierarchy(
        &mut self,
        snapshot: &AnalysisSnapshot,
        source: SourceId,
        span: Span,
        is_cancelled: impl Fn() -> bool + 'static,
    ) -> Result<Vec<Node>, ProjectAnalysisError> {
        let Some(project) = snapshot.workspace.project_for_source(source) else {
            return Ok(Vec::new());
        };
        let (cone, sources) = project.source_cone(source).expect("selected source Cone");
        let file = sources
            .sources
            .iter()
            .find(|file| file.source == source)
            .expect("selected source");
        let revision = snapshot.clone();
        let check: Rc<dyn Fn() -> bool> =
            Rc::new(move || is_cancelled() || revision.check_cancelled().is_err());
        let analysis = self
            .project_declarations(snapshot, project, cone, check.clone())
            .map_err(Clone::clone)?;
        let nodes = analysis
            .call_hierarchy_at(&file.identity, span, check)
            .map_err(|_| ProjectAnalysisError::Cancelled)?;
        let mut result = Vec::new();
        for mut node in nodes {
            let Some(origin) = name_origin(snapshot, project, &node.origin)? else {
                continue;
            };
            if !result.iter().any(|entry: &Node| entry.origin == origin) {
                node.origin = origin;
                result.push(node);
            }
        }
        Ok(result)
    }

    pub fn call_hierarchy(
        &mut self,
        snapshot: &AnalysisSnapshot,
        project_source: SourceId,
        declaration_source: SourceId,
        declaration_span: Span,
        direction: CallHierarchyDirection,
        is_cancelled: impl Fn() -> bool + 'static,
    ) -> Result<Vec<CallHierarchyRelation>, ProjectAnalysisError> {
        let Some(project) = snapshot.workspace.project_for_source(project_source) else {
            return Ok(Vec::new());
        };
        let Some(file) = project.source_file_by_id(declaration_source) else {
            return Ok(Vec::new());
        };
        let origin = SemanticSourceLocation {
            source: file.identity.clone(),
            span: declaration_span,
        };
        let revision = snapshot.clone();
        let check: Rc<dyn Fn() -> bool> =
            Rc::new(move || is_cancelled() || revision.check_cancelled().is_err());
        let outgoing = matches!(direction, CallHierarchyDirection::Outgoing);
        let declaration_cone = project
            .source_cone(declaration_source)
            .map(|(cone, _)| cone);
        let mut names = HashMap::new();
        let mut relations = HashMap::<_, CallHierarchyRelation>::new();
        for (&cone, input) in &project.cones {
            if check() {
                return Err(ProjectAnalysisError::Cancelled);
            }
            if input.source().is_none() || (outgoing && Some(cone) != declaration_cone) {
                continue;
            }
            let analysis = match self.project_declarations(snapshot, project, cone, check.clone()) {
                Ok(analysis) => analysis,
                Err(error) if error.is_cancelled() => return Err(ProjectAnalysisError::Cancelled),
                Err(_) => continue,
            };
            let bodies = if outgoing {
                let primary = analysis
                    .call_hierarchy_body(&origin, check.clone())
                    .map_err(|_| ProjectAnalysisError::Cancelled)?;
                let mut bodies = Vec::new();
                for body in analysis.source_bodies() {
                    if check() {
                        return Err(ProjectAnalysisError::Cancelled);
                    }
                    if primary
                        .as_ref()
                        .is_some_and(|primary| primary.id == body.id)
                    {
                        continue;
                    }
                    if body.origin.source == origin.source
                        && let Some(owner) = analysis.call_hierarchy_owner(&body)
                        && names_match(snapshot, project, &mut names, &owner.origin, &origin)?
                    {
                        bodies.push(body);
                    }
                }
                bodies.extend(primary);
                bodies
            } else {
                analysis.source_bodies()
            };
            for body in bodies {
                if body.callee().is_none() {
                    continue;
                }
                let calls = analysis
                    .call_hierarchy_calls(&body, check.clone())
                    .map_err(|_| ProjectAnalysisError::Cancelled)?;
                for edge in calls {
                    let (here, mut other) = if outgoing {
                        (edge.caller, edge.callee)
                    } else {
                        (edge.callee, edge.caller)
                    };
                    if here.origin.source != origin.source
                        || !names_match(snapshot, project, &mut names, &here.origin, &origin)?
                    {
                        continue;
                    }
                    let Some(other_origin) = name_origin(snapshot, project, &other.origin)? else {
                        continue;
                    };
                    other.origin = other_origin.clone();
                    let relation = relations
                        .entry(other_origin)
                        .or_insert(CallHierarchyRelation {
                            declaration: other,
                            sites: Vec::new(),
                        });
                    relation.sites.push(edge.site);
                }
            }
        }
        for relation in relations.values_mut() {
            relation
                .sites
                .sort_by_key(|site| (site.span.start, site.span.end));
            relation.sites.dedup();
        }
        Ok(relations.into_values().collect())
    }
}

fn names_match(
    snapshot: &AnalysisSnapshot,
    project: &ProjectInput,
    names: &mut HashMap<SemanticSourceLocation, Option<SemanticSourceLocation>>,
    observed: &SemanticSourceLocation,
    wanted: &SemanticSourceLocation,
) -> Result<bool, ProjectAnalysisError> {
    if let Some(origin) = names.get(observed) {
        return Ok(origin.as_ref() == Some(wanted));
    }
    let origin = name_origin(snapshot, project, observed)?;
    let matches = origin.as_ref() == Some(wanted);
    names.insert(observed.clone(), origin);
    Ok(matches)
}

pub(super) fn name_origin(
    snapshot: &AnalysisSnapshot,
    project: &ProjectInput,
    origin: &SemanticSourceLocation,
) -> Result<Option<SemanticSourceLocation>, ProjectAnalysisError> {
    let Some(file) = project.source_file(&origin.source) else {
        return Ok(None);
    };
    let Some(source) = snapshot.source(file.source)? else {
        return Ok(None);
    };
    let span = source
        .declaration_name_span(origin.span)
        .map_err(|error| {
            ProjectAnalysisError::Declarations(DeclarationAnalysisError::SourceLength(*error))
        })?
        .unwrap_or(origin.span);
    Ok(Some(SemanticSourceLocation {
        source: origin.source.clone(),
        span,
    }))
}
