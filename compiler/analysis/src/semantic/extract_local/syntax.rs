use scoop_source::{SourceEdit, Span};
use scoop_syntax::{SyntaxKind as K, SyntaxNode};

use crate::SourceInput;

use super::contains;

pub(super) struct Selection {
    pub expression: Span,
    pub call: Option<Span>,
    insertion: u32,
    name: String,
    kind: ExtractionKind,
    indented_lines: Vec<u32>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ExtractionKind {
    Variable,
    FunctionExpression,
    FunctionStatements,
}

pub(super) struct Extraction {
    pub source_edit: SourceEdit,
    pub initializer: Span,
    pub replacement: Span,
    original: Span,
    indented_lines: Vec<u32>,
}

impl Selection {
    pub fn find(source: &SourceInput, range: Span) -> Option<Self> {
        let expression = trimmed_range(source, range)?;
        let root = source.parse().ok()?.syntax();
        let statement = enclosing_statement(&root, expression)?;
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
        Some(Self {
            expression,
            call,
            insertion: node_span(&statement).start,
            name: fresh_name(&root, "extracted")?,
            kind: ExtractionKind::Variable,
            indented_lines: Vec::new(),
        })
    }

    pub fn find_function(source: &SourceInput, range: Span) -> Option<Self> {
        let selected = trimmed_range(source, range)?;
        let text = source.text.as_str();
        let root = source.parse().ok()?.syntax();
        let (insertion, kind, indented_lines) = if whole_statements(&root, selected) {
            let lines = text[selected.start as usize..selected.end as usize]
                .bytes()
                .enumerate()
                .filter_map(|(index, byte)| {
                    let offset = selected.start + index as u32 + 1;
                    (offset < selected.end
                        && (byte == b'\n'
                            || (byte == b'\r' && text.as_bytes()[offset as usize] != b'\n')))
                        .then_some(offset)
                })
                .filter(|&offset| {
                    root.token_at_offset(offset.into())
                        .right_biased()
                        .is_some_and(|token| {
                            token.kind() == K::Whitespace
                                || u32::from(token.text_range().start()) == offset
                        })
                })
                .collect();
            (selected.start, ExtractionKind::FunctionStatements, lines)
        } else {
            let statement = enclosing_statement(&root, selected)?;
            // Checked expression facts will reject any selection that does
            // not exactly match one expression in the source AST.
            (
                node_span(&statement).start,
                ExtractionKind::FunctionExpression,
                Vec::new(),
            )
        };
        if root
            .descendants_with_tokens()
            .filter_map(|item| item.into_token())
            .any(|token| {
                token.kind() == K::KwReturn
                    && selected.start <= u32::from(token.text_range().start())
                    && u32::from(token.text_range().end()) <= selected.end
            })
        {
            return None;
        }
        Some(Self {
            expression: selected,
            call: None,
            insertion,
            name: fresh_name(&root, "extractedFunction")?,
            kind,
            indented_lines,
        })
    }

    pub const fn kind(&self) -> ExtractionKind {
        self.kind
    }

    pub fn edit(&self, source: &SourceInput, annotation: Option<&str>) -> Option<Extraction> {
        let text = source.text.as_str();
        let insertion = self.insertion as usize;
        let start = self.expression.start as usize;
        let end = self.expression.end as usize;
        let line = text[..insertion].rsplit(['\r', '\n']).next()?;
        let indent = &line[..line.len() - line.trim_start_matches([' ', '\t']).len()];
        let starts_line = line.chars().all(|ch| matches!(ch, ' ' | '\t'));
        let newline = text.find(['\r', '\n']).map_or("\n", |offset| {
            if text[offset..].starts_with("\r\n") {
                "\r\n"
            } else {
                &text[offset..offset + 1]
            }
        });
        let separator = if starts_line {
            format!("{newline}{line}")
        } else {
            "; ".to_owned()
        };
        let mut replacement = match self.kind {
            ExtractionKind::Variable => format!("val {}", self.name),
            ExtractionKind::FunctionExpression | ExtractionKind::FunctionStatements => {
                format!("fun {}()", self.name)
            }
        };
        if let Some(annotation) = annotation {
            replacement.push_str(": ");
            replacement.push_str(annotation);
        }
        let statements = self.kind == ExtractionKind::FunctionStatements;
        if statements {
            replacement.push_str(" {");
            replacement.push_str(newline);
            replacement.push_str(indent);
            replacement.push_str("    ");
        } else {
            replacement.push_str(" = ");
        }
        let initializer_start = u32::try_from(insertion + replacement.len()).ok()?;
        let mut from = start;
        for &offset in &self.indented_lines {
            replacement.push_str(&text[from..offset as usize]);
            replacement.push_str("    ");
            from = offset as usize;
        }
        replacement.push_str(&text[from..end]);
        let initializer_end = u32::try_from(insertion + replacement.len()).ok()?;
        if statements {
            replacement.push_str(newline);
            replacement.push_str(indent);
            replacement.push('}');
        }
        replacement.push_str(&separator);
        replacement.push_str(&text[insertion..start]);
        let use_start = u32::try_from(insertion + replacement.len()).ok()?;
        replacement.push_str(&self.name);
        if self.kind != ExtractionKind::Variable {
            replacement.push_str("()");
        }
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
            indented_lines: self.indented_lines.clone(),
        })
    }
}

impl Extraction {
    pub fn map(&self, span: Span) -> Option<Span> {
        if contains(self.original, span) {
            let indentation_at = |offset, end: bool| {
                4 * self
                    .indented_lines
                    .partition_point(|&line| line < offset || (!end && line == offset))
                    as u32
            };
            return Some(Span::new(
                self.initializer.start
                    + (span.start - self.original.start)
                    + indentation_at(span.start, false),
                self.initializer.start
                    + (span.end - self.original.start)
                    + indentation_at(span.end, true),
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

fn trimmed_range(source: &SourceInput, range: Span) -> Option<Span> {
    let selected = source
        .text
        .as_str()
        .get(range.start as usize..range.end as usize)?;
    let trimmed = selected.trim();
    if trimmed.is_empty() {
        return None;
    }
    let start = range.start + (selected.len() - selected.trim_start().len()) as u32;
    Some(Span::new(start, start + trimmed.len() as u32))
}

fn whole_statements(root: &SyntaxNode, range: Span) -> bool {
    root.token_at_offset(range.start.into())
        .right_biased()
        .into_iter()
        .flat_map(|token| token.parent_ancestors())
        .filter(|node| node.kind() == K::Statement && node_span(node).start == range.start)
        .filter_map(|node| node.parent())
        .filter(|node| node.kind() == K::Block)
        .any(|block| {
            block.children().any(|node| {
                if node.kind() != K::Statement {
                    return false;
                }
                let span = node_span(&node);
                span.start >= range.start && span.end == range.end
            })
        })
}

fn enclosing_statement(root: &SyntaxNode, range: Span) -> Option<SyntaxNode> {
    root.token_at_offset(range.start.into())
        .right_biased()?
        .parent_ancestors()
        .find(|node| {
            node.kind() == K::Statement
                && node.parent().is_some_and(|p| p.kind() == K::Block)
                && contains(node_span(node), range)
        })
}

fn fresh_name(root: &SyntaxNode, stem: &str) -> Option<String> {
    let names = root
        .descendants_with_tokens()
        .filter_map(|item| item.into_token())
        .filter(|token| matches!(token.kind(), K::Ident | K::EscapedIdent))
        .collect::<Vec<_>>();
    std::iter::once(stem.to_owned())
        .chain((2..).map(|n| format!("{stem}{n}")))
        .find(|name| {
            !names
                .iter()
                .any(|token| token.text().trim_matches('`') == name)
        })
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
