use scoop_ast::{ClassDecl, ParameterSyntax, SourceFile};
use scoop_identity::SourceIdentity;

use super::*;

type Override = (SemanticSourceLocation, SemanticSourceLocation);

pub(super) struct OverrideSet<'a> {
    pub methods: Vec<&'a FunctionDecl>,
    origins: HashSet<SemanticSourceLocation>,
    edges: Vec<Override>,
}

impl<'a> OverrideSet<'a> {
    pub fn select(
        analysis: &mut DeclarationAnalysis,
        input: &'a SourceInput,
        origin: SemanticSourceLocation,
        check: &dyn Fn() -> bool,
    ) -> Result<Option<Self>, ProjectAnalysisError> {
        let Some(ast) = input.parse().ok().and_then(|parsed| parsed.ast()) else {
            return Ok(None);
        };
        let edges = analysis
            .member_override_relations(check)
            .map_err(|_| ProjectAnalysisError::Cancelled)?;
        let mut origins = HashSet::from([origin.clone()]);
        let mut pending = vec![origin.clone()];
        while let Some(current) = pending.pop() {
            if check() {
                return Err(ProjectAnalysisError::Cancelled);
            }
            for (parent, child) in edges {
                let other = if &current == parent {
                    child
                } else if &current == child {
                    parent
                } else {
                    continue;
                };
                if origins.insert(other.clone()) {
                    pending.push(other.clone());
                }
            }
        }
        let Some(mut methods) = origins
            .iter()
            .map(|method| {
                if method.source != origin.source {
                    return None;
                }
                let declaration = declared_members(ast)
                    .find(|declaration| declaration.name.span == method.span)?;
                if declaration.is_override && !edges.iter().any(|(_, child)| child == method) {
                    return None;
                }
                Some(declaration)
            })
            .collect::<Option<Vec<_>>>()
        else {
            return Ok(None);
        };
        methods.sort_unstable_by_key(|method| method.span.start);
        let edges = incident_edges(edges, &origins).cloned().collect();
        Ok(Some(Self {
            methods,
            origins,
            edges,
        }))
    }

    pub fn preserved(
        &self,
        analysis: &mut DeclarationAnalysis,
        edits: &[SourceEdit],
        source: &SourceIdentity,
        check: &dyn Fn() -> bool,
    ) -> Result<bool, ProjectAnalysisError> {
        let map = |origin: &SemanticSourceLocation| {
            Some(SemanticSourceLocation {
                source: origin.source.clone(),
                span: if &origin.source == source {
                    updated_span(origin.span, edits)?
                } else {
                    origin.span
                },
            })
        };
        let Some(origins) = self.origins.iter().map(map).collect::<Option<HashSet<_>>>() else {
            return Ok(false);
        };
        let Some(expected) = self
            .edges
            .iter()
            .map(|(parent, child)| Some((map(parent)?, map(child)?)))
            .collect::<Option<HashSet<_>>>()
        else {
            return Ok(false);
        };
        let actual = analysis
            .member_override_relations(check)
            .map_err(|_| ProjectAnalysisError::Cancelled)?;
        Ok(expected == incident_edges(actual, &origins).cloned().collect())
    }
}

fn incident_edges<'a>(
    edges: &'a [Override],
    origins: &'a HashSet<SemanticSourceLocation>,
) -> impl Iterator<Item = &'a Override> {
    edges
        .iter()
        .filter(|(parent, child)| origins.contains(parent) || origins.contains(child))
}

pub(super) fn member_at(input: &SourceInput, selection: Span) -> Option<&FunctionDecl> {
    declared_members(input.parse().ok()?.ast()?).find(|method| inside(method.name.span, selection))
}

fn declared_members(ast: &SourceFile) -> impl Iterator<Item = &FunctionDecl> {
    let classes = ast
        .declarations
        .iter()
        .filter_map(|declaration| match declaration {
            Decl::Class(class)
                if class.type_params.is_empty() && editable_owner(class.visibility) =>
            {
                Some(class)
            }
            _ => None,
        });
    let interfaces = ast
        .declarations
        .iter()
        .filter_map(|declaration| match declaration {
            Decl::Interface(interface)
                if interface.type_params.is_empty() && editable_owner(interface.visibility) =>
            {
                Some(interface)
            }
            _ => None,
        });
    classes
        .flat_map(ClassDecl::functions)
        .chain(interfaces.flat_map(|interface| interface.methods.iter()))
        .filter(|method| {
            method.params.len() >= 2
                && method.annotations.is_empty()
                && method.type_params.is_empty()
                && method.context_parameters.is_empty()
                && method.operator.is_none()
                && method.infix.is_none()
                && method
                    .params
                    .iter()
                    .all(|parameter| !matches!(parameter.syntax, ParameterSyntax::Vararg { .. }))
        })
}

fn editable_owner(visibility: VisibilitySyntax) -> bool {
    matches!(
        visibility,
        VisibilitySyntax::Omitted
            | VisibilitySyntax::Explicit {
                visibility: DeclaredVisibility::Internal | DeclaredVisibility::Private,
                ..
            }
    )
}
