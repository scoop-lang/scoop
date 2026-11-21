use std::collections::HashSet;

use scoop_ast::{
    ClassDecl, ClassModifier, Decl, DeclaredVisibility, FunctionBody, FunctionDecl, MethodModifier,
    ParameterSyntax, SourceFile, VisibilitySyntax,
};
use scoop_hir::{
    EditingReferenceTarget, EditingTypeIdentity, PublicNominalKindV1, SemanticSourceLocation,
    SourceNominalId,
};
use scoop_source::{SourceEdit, Span};
use scoop_syntax::{SyntaxKind as K, SyntaxNode};

use super::extract_declaration::complete_references;
use super::extract_local::{bindings, contains};
use super::inline_function::source_can_move;
use super::*;
use crate::SnapshotEditError;

mod edits;

#[derive(Clone, Copy)]
pub enum MemberRefactoring {
    ExtractInterface,
    ExtractSuperclass,
    PullUp,
    PushDown,
}

impl SemanticSession {
    pub fn refactor_members(
        &mut self,
        snapshot: &AnalysisSnapshot,
        source: SourceId,
        selection: Span,
        kind: MemberRefactoring,
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
        let Some(owner) = classes(ast).find(|class| contains(class.span, selection)) else {
            return Ok(None);
        };
        let Some(visibility) = class_visibility(owner) else {
            return Ok(None);
        };
        let class_selected = contains(owner.name.span, selection);
        let methods = owner
            .functions()
            .filter(|method| {
                class_selected
                    || contains(method.name.span, selection)
                    || contains(selection, method.span)
            })
            .filter(|method| movable_signature(method))
            .collect::<Vec<_>>();
        if methods.is_empty() {
            return Ok(None);
        }
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
        if !before.unavailable.is_empty() {
            return Ok(None);
        }
        let name = SemanticSourceLocation {
            source: file.identity.clone(),
            span: owner.name.span,
        };
        let [EditingReferenceTarget::Type(EditingTypeIdentity::Nominal(nominal))] =
            before.targets_at(&name)
        else {
            return Ok(None);
        };
        let root = parsed.syntax();
        if !matches!(kind, MemberRefactoring::ExtractInterface) {
            for method in &methods {
                let body = match &method.body {
                    FunctionBody::Block(block) => block.span,
                    FunctionBody::Expr(expr) => expr.span(),
                    FunctionBody::None => return Ok(None),
                };
                if uses_original_receiver(&root, body)
                    || !source_can_move(analysis, &file.identity, body, check.clone())?
                {
                    return Ok(None);
                }
            }
        }
        let parent_classes = || {
            analysis
                .hierarchy_supertypes(*nominal)
                .into_iter()
                .filter(|&parent| {
                    analysis
                        .hierarchy_type(parent)
                        .is_some_and(|ty| ty.kind == PublicNominalKindV1::Class)
                })
                .collect::<Vec<_>>()
        };
        let plan = match kind {
            MemberRefactoring::ExtractInterface => {
                edits::extract_interface(input, &root, owner, visibility, &methods)
            }
            MemberRefactoring::ExtractSuperclass => {
                if !parent_classes().is_empty() || owner.secondary_constructors().next().is_some() {
                    return Ok(None);
                }
                edits::extract_superclass(input, &root, owner, visibility, &methods)
            }
            MemberRefactoring::PullUp | MemberRefactoring::PushDown => {
                let related = match kind {
                    MemberRefactoring::PullUp => parent_classes(),
                    _ => analysis
                        .hierarchy_subtypes(*nominal, &*check)
                        .map_err(|_| ProjectAnalysisError::Cancelled)?
                        .to_vec(),
                };
                let [related] = related.as_slice() else {
                    return Ok(None);
                };
                let Some(target) = editable_class(analysis, ast, *related, &file.identity) else {
                    return Ok(None);
                };
                edits::move_members(input, &root, target, &methods)
            }
        };
        let Some(plan) = plan else {
            return Ok(None);
        };
        let candidate = match snapshot.with_edits(&plan.edits) {
            Ok(candidate) => candidate,
            Err(SnapshotEditError::Cancelled) => return Err(ProjectAnalysisError::Cancelled),
            Err(_) => return Ok(None),
        };
        let mut session = self.for_edits();
        let analysis = session
            .project_declarations(&candidate, project, cone, check.clone())
            .map_err(Clone::clone)?;
        let Some(after) = complete_references(analysis, &file.identity, check.clone())? else {
            return Ok(None);
        };
        if !after.unavailable.is_empty() {
            return Ok(None);
        }
        let Some(expected) = bindings(
            &before,
            |_| true,
            |origin| {
                Some(SemanticSourceLocation {
                    source: origin.source.clone(),
                    span: if origin.source == file.identity {
                        plan.map(origin.span)?
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
        let Some(actual) = bindings(&after, |_| true, |origin| Some(origin.clone()), &*check)?
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
        .then_some(plan.edits))
    }
}

fn classes(ast: &SourceFile) -> impl Iterator<Item = &ClassDecl> {
    ast.declarations
        .iter()
        .filter_map(|declaration| match declaration {
            Decl::Class(class) => Some(class),
            _ => None,
        })
}

fn class_visibility(class: &ClassDecl) -> Option<&'static str> {
    if !class.annotations.is_empty()
        || class.inner_span.is_some()
        || !class.type_params.is_empty()
        || class.modifier == ClassModifier::Sealed
    {
        return None;
    }
    match class.visibility {
        VisibilitySyntax::Omitted => Some(""),
        VisibilitySyntax::Explicit {
            visibility: DeclaredVisibility::Internal,
            ..
        } => Some("internal "),
        VisibilitySyntax::Explicit {
            visibility: DeclaredVisibility::Private,
            ..
        } => Some("private "),
        _ => None,
    }
}

fn movable_signature(method: &FunctionDecl) -> bool {
    method.annotations.is_empty()
        && method.modifier == MethodModifier::Final
        && !method.is_override
        && !method.is_suspend
        && method.context_parameters.is_empty()
        && method.type_params.is_empty()
        && method.operator.is_none()
        && method.infix.is_none()
        && method.return_ty.is_some()
        && method.params.iter().all(|parameter| {
            parameter.annotations.is_empty()
                && matches!(parameter.syntax, ParameterSyntax::Required)
        })
        && !matches!(method.body, FunctionBody::None)
        && matches!(
            method.visibility,
            VisibilitySyntax::Omitted
                | VisibilitySyntax::Explicit {
                    visibility: DeclaredVisibility::Internal | DeclaredVisibility::Public,
                    ..
                }
        )
}

fn editable_class<'a>(
    analysis: &DeclarationAnalysis,
    ast: &'a SourceFile,
    declaration: SourceNominalId,
    source: &scoop_identity::SourceIdentity,
) -> Option<&'a ClassDecl> {
    let related = analysis.hierarchy_type(declaration)?;
    if related.kind != PublicNominalKindV1::Class || &related.origin.source != source {
        return None;
    }
    classes(ast).find(|class| {
        contains(related.origin.span, class.name.span) && class_visibility(class).is_some()
    })
}

fn uses_original_receiver(root: &SyntaxNode, span: Span) -> bool {
    root.descendants_with_tokens()
        .filter_map(|item| item.into_token())
        .any(|token| {
            (token.kind() == K::KwThis || token.kind() == K::Ident && token.text() == "super")
                && contains(
                    span,
                    Span::new(
                        token.text_range().start().into(),
                        token.text_range().end().into(),
                    ),
                )
        })
}
