use std::collections::HashSet;

use scoop_ast::{Decl, DeclaredVisibility, FunctionDecl, ParameterSyntax, VisibilitySyntax};
use scoop_hir::{EditingReferenceIndex, EditingValue, SemanticSourceLocation};
use scoop_source::{SourceEdit, Span};
use scoop_syntax::{SyntaxKind as K, SyntaxNode};

use super::change_signature::checked_direct_calls;
use super::extract_local::{
    preserves_bindings,
    syntax::{Extraction, fresh_name, node_span, trimmed_range},
};
use super::move_declaration::attached_comments;
use super::*;
use crate::{SnapshotEditError, SourceInput};

#[derive(Clone, Copy)]
pub enum DeclarationExtraction {
    Constant,
    Property,
    Parameter,
}

impl SemanticSession {
    pub fn extract_declaration(
        &mut self,
        snapshot: &AnalysisSnapshot,
        source: SourceId,
        selected: Span,
        kind: DeclarationExtraction,
        is_cancelled: impl Fn() -> bool + 'static,
    ) -> Result<Option<SourceEdit>, ProjectAnalysisError> {
        let Some(input) = snapshot.source(source)? else {
            return Ok(None);
        };
        let Some(selection) = trimmed_range(input, selected) else {
            return Ok(None);
        };
        let Some(site) = ExtractionSite::find(input, selection, kind) else {
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
        let cancellation = check.clone();
        let Some(body) = analysis
            .source_body_at(&file.identity, selection, move || cancellation())
            .map_err(|_| ProjectAnalysisError::Cancelled)?
        else {
            return Ok(None);
        };
        let Some((value_type, _)) = analysis.expression_type_for_edit(body.id, selection) else {
            return Ok(None);
        };
        let Some(spelling) = analysis.expression_type_spelling(body.id, selection) else {
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
            .filter(|call| contains(selection, call.span))
            .filter_map(|call| call.callee)
            .collect::<Vec<_>>();
        if callees
            .into_iter()
            .any(|callee| analysis.is_source_location_call(callee))
            || analysis.has_source_location_default(body.id, selection)
        {
            return Ok(None);
        }
        let Some(before) = complete_references(analysis, &file.identity, check.clone())? else {
            return Ok(None);
        };
        if let Some(function) = site.function {
            let origin = SemanticSourceLocation {
                source: file.identity.clone(),
                span: function,
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
        }
        let Some(edit) = site.edit(input, selection, &spelling, kind) else {
            return Ok(None);
        };
        let mut folded_operators = HashSet::new();
        if matches!(kind, DeclarationExtraction::Constant) {
            let cancellation = check.clone();
            let EditingValue::Known(references) = analysis
                .callable_references(body.id, move || cancellation())
                .map_err(|_| ProjectAnalysisError::Cancelled)?
            else {
                return Ok(None);
            };
            let operators = references
                .iter()
                .filter(|reference| contains(selection, reference.span))
                .map(|reference| (reference.span, reference.target))
                .collect::<Vec<_>>();
            for (span, target) in operators {
                if input
                    .text
                    .as_str()
                    .get(span.start as usize..span.end as usize)
                    .is_some_and(|symbol| analysis.is_constant_integer_operator(target, symbol))
                    && let Some(span) = edit.map(span)
                {
                    folded_operators.insert(span);
                }
            }
        }
        let candidate = match snapshot.with_edits(std::slice::from_ref(&edit.source_edit)) {
            Ok(candidate) => candidate,
            Err(SnapshotEditError::Cancelled) => return Err(ProjectAnalysisError::Cancelled),
            Err(_) => return Ok(None),
        };
        let mut session = self.for_edits();
        let analysis = session
            .project_declarations(&candidate, project, cone, check.clone())
            .map_err(Clone::clone)?;
        let cancellation = check.clone();
        let Some(body) = analysis
            .source_body_at(&file.identity, edit.replacement, move || cancellation())
            .map_err(|_| ProjectAnalysisError::Cancelled)?
        else {
            return Ok(None);
        };
        if analysis
            .expression_type_for_edit(body.id, edit.replacement)
            .is_none_or(|(ty, _)| ty != value_type)
        {
            return Ok(None);
        }
        let Some(after) = complete_references(analysis, &file.identity, check.clone())? else {
            return Ok(None);
        };
        if !preserves_bindings(
            &before,
            &after,
            &file.identity,
            &edit,
            &folded_operators,
            &*check,
        )? {
            return Ok(None);
        }
        Ok(Some(edit.source_edit))
    }
}

fn complete_references(
    analysis: &mut DeclarationAnalysis,
    source: &scoop_identity::SourceIdentity,
    check: Rc<dyn Fn() -> bool>,
) -> Result<Option<EditingReferenceIndex>, ProjectAnalysisError> {
    let index = analysis
        .reference_index(move || check())
        .map_err(|_| ProjectAnalysisError::Cancelled)?;
    Ok((index.declarations_available
        && index
            .unavailable
            .iter()
            .all(|region| &region.source != source))
    .then(|| index.clone()))
}

struct ExtractionSite {
    insertion: u32,
    name: String,
    function: Option<Span>,
    parameter_separator: &'static str,
}

impl ExtractionSite {
    fn find(input: &SourceInput, selection: Span, kind: DeclarationExtraction) -> Option<Self> {
        let parsed = input.parse().ok()?;
        let root = parsed.syntax();
        let file = parsed.ast()?;
        let containing = file
            .declarations
            .iter()
            .find(|declaration| contains(declaration.span(), selection))?;
        let (insertion, stem, function, parameter_separator) = match kind {
            DeclarationExtraction::Constant => (
                attached_comments(input, containing.span()).start,
                "extractedConstant",
                None,
                "",
            ),
            DeclarationExtraction::Property => {
                let member = root
                    .token_at_offset(selection.start.into())
                    .right_biased()?
                    .parent_ancestors()
                    .find(|node| {
                        node.kind() == K::Member
                            && node
                                .parent()
                                .is_some_and(|parent| parent.kind() == K::ClassBody)
                            && node.children().any(|child| child.kind() == K::FunctionDecl)
                    })?;
                (
                    attached_comments(input, node_span(&member)).start,
                    "extractedProperty",
                    None,
                    "",
                )
            }
            DeclarationExtraction::Parameter => {
                let Decl::Function(function) = containing else {
                    return None;
                };
                if !ordinary_private(function) || !stable_default(input, &root, function, selection)
                {
                    return None;
                }
                let declaration = root
                    .token_at_offset(function.name.span.start.into())
                    .right_biased()?
                    .parent_ancestors()
                    .find(|node| node.kind() == K::FunctionDecl)?;
                let parameters = declaration
                    .children()
                    .find(|node| node.kind() == K::ParameterList)?;
                let end = parameters
                    .children_with_tokens()
                    .filter_map(|item| item.into_token())
                    .find(|token| token.kind() == K::RParen)?
                    .text_range()
                    .start()
                    .into();
                let separator = if let Some(last) = function.params.last() {
                    let tail = input
                        .text
                        .as_str()
                        .get(last.span.end as usize..end as usize)?;
                    if tail.trim_start().starts_with(',') {
                        " "
                    } else {
                        ", "
                    }
                } else {
                    ""
                };
                (
                    end,
                    "extractedParameter",
                    Some(function.name.span),
                    separator,
                )
            }
        };
        Some(Self {
            insertion,
            name: fresh_name(&root, stem)?,
            function,
            parameter_separator,
        })
    }

    fn edit(
        &self,
        input: &SourceInput,
        selected: Span,
        ty: &str,
        kind: DeclarationExtraction,
    ) -> Option<Extraction> {
        let name = &self.name;
        let (prefix, suffix) = match kind {
            DeclarationExtraction::Constant => (
                format!("private const val {name}: {ty} = "),
                separator(input, self.insertion),
            ),
            DeclarationExtraction::Property => (
                format!("private val {name}: {ty} get() = "),
                separator(input, self.insertion),
            ),
            DeclarationExtraction::Parameter => (
                format!("{}{name}: {ty} = ", self.parameter_separator),
                String::new(),
            ),
        };
        Extraction::declaration(input, selected, self.insertion, prefix, &suffix, name)
    }
}

fn ordinary_private(function: &FunctionDecl) -> bool {
    function.annotations.is_empty()
        && function.receiver_ty.is_none()
        && matches!(
            function.visibility,
            VisibilitySyntax::Explicit {
                visibility: DeclaredVisibility::Private,
                ..
            }
        )
        && function
            .params
            .iter()
            .all(|param| !matches!(param.syntax, ParameterSyntax::Vararg { .. }))
}

fn stable_default(
    input: &SourceInput,
    root: &SyntaxNode,
    function: &FunctionDecl,
    selected: Span,
) -> bool {
    let Some(token) = root.token_at_offset(selected.start.into()).right_biased() else {
        return false;
    };
    if u32::from(token.text_range().end()) != selected.end {
        return false;
    }
    match token.kind() {
        K::KwTrue | K::KwFalse | K::Integer | K::Float | K::Char | K::String => true,
        K::Ident | K::EscapedIdent => {
            let value = &input.text.as_str()[selected.start as usize..selected.end as usize];
            scoop_parser::identifier_name(value).is_some_and(|name| {
                function
                    .params
                    .iter()
                    .any(|parameter| parameter.name.text == name)
            })
        }
        _ => false,
    }
}

fn contains(outer: Span, inner: Span) -> bool {
    outer.start <= inner.start && inner.end <= outer.end
}

fn separator(input: &SourceInput, insertion: u32) -> String {
    let text = input.text.as_str();
    let line = text[..insertion as usize]
        .rsplit(['\r', '\n'])
        .next()
        .unwrap_or_default();
    if line.chars().all(|ch| matches!(ch, ' ' | '\t')) {
        let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
        format!("{newline}{line}")
    } else {
        "; ".to_owned()
    }
}
