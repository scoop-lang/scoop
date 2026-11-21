use scoop_source::{SourceEdit, Span};
use scoop_syntax::{SyntaxKind as K, SyntaxNode};

use crate::SourceInput;

use super::contains;

pub(super) struct Selection {
    pub expression: Span,
    pub call: Option<Span>,
    insertion: u32,
    name: String,
}

pub(super) struct Extraction {
    pub source_edit: SourceEdit,
    pub initializer: Span,
    pub replacement: Span,
    original: Span,
}

impl Selection {
    pub fn find(source: &SourceInput, range: Span) -> Option<Self> {
        let text = source.text.as_str();
        let selected = text.get(range.start as usize..range.end as usize)?;
        let leading = selected.len() - selected.trim_start().len();
        let start = range.start + leading as u32;
        let end = start + selected.trim().len() as u32;
        let expression = Span::new(start, end);
        if start == end {
            return None;
        }
        let root = source.parse().ok()?.syntax();
        let statement = root
            .token_at_offset(start.into())
            .right_biased()?
            .parent_ancestors()
            .find(|node| {
                node.kind() == K::Statement
                    && node.parent().is_some_and(|p| p.kind() == K::Block)
                    && contains(node_span(node), expression)
            })?;
        let opening = statement
            .children_with_tokens()
            .find(|item| !item.kind().is_trivia())?;
        let roots = statement
            .children()
            .filter(|node| node.kind() == K::Expression)
            .collect::<Vec<_>>();
        let [outer] = roots.as_slice() else {
            return None;
        };
        let whole_value = matches!(opening.kind(), K::KwVal | K::KwVar | K::KwReturn);
        let call = if whole_value && node_span(outer) == expression {
            None
        } else if (whole_value || opening.kind() == K::Expression)
            && first_direct_argument(outer, expression)
        {
            Some(node_span(outer))
        } else {
            return None;
        };
        let insertion = statement
            .descendants_with_tokens()
            .filter_map(|item| item.into_token())
            .find(|token| !token.kind().is_trivia())
            .map(|token| u32::from(token.text_range().start()))?;
        let names = root
            .descendants_with_tokens()
            .filter_map(|item| item.into_token())
            .filter(|token| matches!(token.kind(), K::Ident | K::EscapedIdent))
            .collect::<Vec<_>>();
        let name = std::iter::once("extracted".to_owned())
            .chain((2..).map(|n| format!("extracted{n}")))
            .find(|name| {
                !names
                    .iter()
                    .any(|token| token.text().trim_matches('`') == name)
            })?;
        Some(Self {
            expression,
            call,
            insertion,
            name,
        })
    }

    pub fn edit(&self, source: &SourceInput, annotation: Option<&str>) -> Option<Extraction> {
        let text = source.text.as_str();
        let insertion = self.insertion as usize;
        let start = self.expression.start as usize;
        let end = self.expression.end as usize;
        let line = text[..insertion].rsplit(['\r', '\n']).next()?;
        let separator = if line.chars().all(|ch| matches!(ch, ' ' | '\t')) {
            let newline = text.find(['\r', '\n']).map_or("\n", |offset| {
                if text[offset..].starts_with("\r\n") {
                    "\r\n"
                } else {
                    &text[offset..offset + 1]
                }
            });
            format!("{newline}{line}")
        } else {
            "; ".to_owned()
        };
        let mut replacement = format!("val {}", self.name);
        if let Some(annotation) = annotation {
            replacement.push_str(": ");
            replacement.push_str(annotation);
        }
        replacement.push_str(" = ");
        let initializer_start = u32::try_from(insertion + replacement.len()).ok()?;
        replacement.push_str(&text[start..end]);
        let initializer_end = u32::try_from(insertion + replacement.len()).ok()?;
        replacement.push_str(&separator);
        replacement.push_str(&text[insertion..start]);
        let use_start = u32::try_from(insertion + replacement.len()).ok()?;
        replacement.push_str(&self.name);
        let use_end = u32::try_from(insertion + replacement.len()).ok()?;
        Some(Extraction {
            source_edit: SourceEdit {
                source: source.version,
                span: Span::new(self.insertion, self.expression.end),
                replacement,
            },
            initializer: Span::new(initializer_start, initializer_end),
            replacement: Span::new(use_start, use_end),
            original: self.expression,
        })
    }
}

impl Extraction {
    pub fn map(&self, span: Span) -> Option<Span> {
        if contains(self.original, span) {
            return Some(Span::new(
                self.initializer.start + (span.start - self.original.start),
                self.initializer.start + (span.end - self.original.start),
            ));
        }
        let map = |offset, is_end| {
            let shift = if offset < self.source_edit.span.start
                || (is_end && offset == self.source_edit.span.start)
            {
                0
            } else if offset < self.original.start || (is_end && offset == self.original.start) {
                i64::from(self.replacement.start) - i64::from(self.original.start)
            } else if offset >= self.original.end {
                i64::from(self.replacement.end) - i64::from(self.original.end)
            } else {
                return None;
            };
            u32::try_from(i64::from(offset) + shift).ok()
        };
        Some(Span::new(map(span.start, false)?, map(span.end, true)?))
    }
}

fn first_direct_argument(outer: &SyntaxNode, expression: Span) -> bool {
    let Some(atom) = outer
        .children()
        .find(|node| node.kind() == K::Atom && node_span(node) == node_span(outer))
    else {
        return false;
    };
    let Some(list) = atom.children().find(|node| node.kind() == K::ArgumentList) else {
        return false;
    };
    let significant = atom
        .descendants_with_tokens()
        .filter_map(|part| part.into_token())
        .take_while(|token| token.text_range().start() < list.text_range().start())
        .filter(|token| !token.kind().is_trivia())
        .collect::<Vec<_>>();
    if !matches!(
        significant.first().map(|token| token.kind()),
        Some(K::Ident | K::EscapedIdent)
    ) {
        return false;
    }
    if significant.len() != 1
        && !atom.children().any(|child| {
            child.kind() == K::TypeArgumentList
                && child.text_range().end() <= list.text_range().start()
                && significant[1].text_range().start() == child.text_range().start()
                && significant
                    .last()
                    .is_some_and(|token| token.text_range().end() == child.text_range().end())
        })
    {
        return false;
    }
    list.children()
        .find(|node| node.kind() == K::Argument)
        .and_then(|argument| {
            argument
                .children()
                .find(|node| node.kind() == K::Expression)
        })
        .is_some_and(|argument| node_span(&argument) == expression)
}

fn node_span(node: &SyntaxNode) -> Span {
    let mut tokens = node
        .descendants_with_tokens()
        .filter_map(|part| part.into_token())
        .filter(|token| !token.kind().is_trivia());
    let Some(first) = tokens.next() else {
        return Span::new(
            node.text_range().start().into(),
            node.text_range().end().into(),
        );
    };
    let last = tokens.last().unwrap_or_else(|| first.clone());
    Span::new(
        first.text_range().start().into(),
        last.text_range().end().into(),
    )
}
