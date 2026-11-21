use std::collections::HashMap;
use std::fmt::Write;

use scoop_ast::{FunctionDecl, ParameterSyntax};
use scoop_hir::{EditingAppliedCall, EditingReferenceIndex};
use scoop_identity::SourceIdentity;
use scoop_source::{SourceEdit, Span};
use scoop_syntax::{SyntaxKind as K, SyntaxNode};

use crate::SourceInput;

use super::super::extract_local::{
    contains,
    syntax::{fresh_name, node_span},
};
use super::super::inline_local::updated_span;

pub(super) struct InlineCall<'a> {
    pub input: &'a SourceInput,
    pub root: &'a SyntaxNode,
    pub source: &'a SourceIdentity,
    pub references: &'a EditingReferenceIndex,
    pub call: &'a EditingAppliedCall,
    pub function: &'a FunctionDecl,
    pub parameter_types: Vec<String>,
    pub result_type: &'a str,
    pub statements: Option<Span>,
    pub expression: Span,
}

pub(super) struct InlineEdit {
    pub source_edit: SourceEdit,
    copies: Vec<Copy>,
    parameters: HashMap<Span, Span>,
}

struct Copy {
    original: Span,
    start: u32,
    replacements: Vec<SourceEdit>,
}

impl InlineEdit {
    pub fn build(input: InlineCall<'_>) -> Option<Self> {
        let text = input.input.text.as_str();
        let call = input.call;
        let line = text
            .get(..call.span.start as usize)?
            .rsplit(['\r', '\n'])
            .next()?;
        let indent = &line[..line.len() - line.trim_start_matches([' ', '\t']).len()];
        let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
        let names = (1..=input.function.params.len())
            .map(|index| fresh_name(input.root, &format!("inlineArgument{index}Value")))
            .collect::<Option<Vec<_>>>()?;
        if names.len() != input.parameter_types.len() {
            return None;
        }
        let mut edit = Self {
            source_edit: SourceEdit {
                source: input.input.version,
                span: call.span,
                replacement: format!("(when {{{newline}{indent}    else -> {{"),
            },
            copies: Vec::new(),
            parameters: HashMap::new(),
        };
        let next = format!("{newline}{indent}        ");
        let mut has_argument = vec![false; names.len()];
        for argument in &call.arguments {
            if std::mem::replace(has_argument.get_mut(argument.parameter)?, true) {
                return None;
            }
            let value = if let Some(label) = argument.name_span {
                let node = input
                    .root
                    .token_at_offset(label.start.into())
                    .right_biased()?
                    .parent_ancestors()
                    .find(|node| {
                        node.kind() == K::Argument && contains(node_span(node), argument.span)
                    })?;
                let value = node
                    .children()
                    .find(|child| child.kind() == K::Expression)?;
                node_span(&value)
            } else {
                argument.span
            };
            edit.argument(
                &input,
                &next,
                argument.parameter,
                &names[argument.parameter],
                value,
            )?;
        }
        for (index, (parameter, given)) in
            input.function.params.iter().zip(has_argument).enumerate()
        {
            if given {
                continue;
            }
            let ParameterSyntax::Default { expression, .. } = &parameter.syntax else {
                return None;
            };
            edit.argument(&input, &next, index, &names[index], expression.span())?;
        }
        if let Some(statements) = input.statements {
            edit.source_edit.replacement.push_str(&next);
            edit.copy(&input, statements)?;
        }
        let name = fresh_name(input.root, "inlineResult")?;
        write!(
            &mut edit.source_edit.replacement,
            "{next}val {name}: {} = ",
            input.result_type
        )
        .ok()?;
        edit.copy(&input, input.expression)?;
        write!(
            &mut edit.source_edit.replacement,
            "{next}{name}{newline}{indent}    }}{newline}{indent}}})"
        )
        .ok()?;

        let loses_comment = input
            .root
            .descendants_with_tokens()
            .filter_map(|element| element.into_token())
            .filter(|token| {
                matches!(
                    token.kind(),
                    K::LineComment | K::BlockComment | K::DocLineComment | K::DocBlockComment
                )
            })
            .any(|token| {
                let span = Span::new(
                    token.text_range().start().into(),
                    token.text_range().end().into(),
                );
                contains(call.span, span) && !edit.copies(span)
            });
        edit.end()?;
        (!loses_comment).then_some(edit)
    }

    fn end(&self) -> Option<u32> {
        self.source_edit
            .span
            .start
            .checked_add(u32::try_from(self.source_edit.replacement.len()).ok()?)
    }

    fn argument(
        &mut self,
        input: &InlineCall<'_>,
        next: &str,
        index: usize,
        name: &str,
        value: Span,
    ) -> Option<()> {
        write!(&mut self.source_edit.replacement, "{next}val ").ok()?;
        let start = self.end()?;
        self.source_edit.replacement.push_str(name);
        let end = self.end()?;
        self.parameters.insert(
            input.function.params[index].name.span,
            Span::new(start, end),
        );
        write!(
            &mut self.source_edit.replacement,
            ": {} = ",
            input.parameter_types[index]
        )
        .ok()?;
        self.copy(input, value)
    }

    fn copy(&mut self, input: &InlineCall<'_>, span: Span) -> Option<()> {
        let text = input.input.text.as_str();
        let mut renamed = std::collections::BTreeMap::new();
        for reference in input.references.occurrences() {
            if reference.is_declaration
                || &reference.origin.source != input.source
                || !contains(span, reference.origin.span)
            {
                continue;
            }
            let Some(definition) = input.references.definitions.get(&reference.target) else {
                continue;
            };
            if &definition.source == input.source
                && let Some(name) = self.parameters.get(&definition.span)
            {
                let start = (name.start - self.source_edit.span.start) as usize;
                let end = (name.end - self.source_edit.span.start) as usize;
                renamed.insert(
                    reference.origin.span.start,
                    SourceEdit {
                        source: input.input.version,
                        span: reference.origin.span,
                        replacement: self.source_edit.replacement.get(start..end)?.to_owned(),
                    },
                );
            }
        }
        let replacements = renamed.into_values().collect::<Vec<_>>();
        let start = self.end()?;
        let mut from = span.start as usize;
        for edit in &replacements {
            self.source_edit
                .replacement
                .push_str(text.get(from..edit.span.start as usize)?);
            self.source_edit.replacement.push_str(&edit.replacement);
            from = edit.span.end as usize;
        }
        self.source_edit
            .replacement
            .push_str(text.get(from..span.end as usize)?);
        self.copies.push(Copy {
            original: span,
            start,
            replacements,
        });
        Some(())
    }

    pub fn replacement(&self) -> Span {
        Span::new(
            self.source_edit.span.start,
            self.end().expect("source edit fits input offsets"),
        )
    }

    pub fn copied_spans(&self) -> impl Iterator<Item = Span> + '_ {
        self.copies.iter().map(|copy| copy.original)
    }

    pub fn copies(&self, span: Span) -> bool {
        self.copied_spans().any(|copy| contains(copy, span))
    }

    pub fn copied_position(&self, span: Span) -> Option<Span> {
        if let Some(&parameter) = self.parameters.get(&span) {
            return Some(parameter);
        }
        let copy = self
            .copies
            .iter()
            .find(|copy| contains(copy.original, span))?;
        let span = updated_span(span, &copy.replacements)?;
        Some(Span::new(
            copy.start + span.start - copy.original.start,
            copy.start + span.end - copy.original.start,
        ))
    }
}
