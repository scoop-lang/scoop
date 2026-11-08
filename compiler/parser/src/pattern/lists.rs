use super::*;
use crate::parser::ListKind;
use scoop_syntax::SyntaxKind;

enum PatternItem {
    Value(Pattern),
    Rest(Span),
}

enum NamedItem {
    Field(FieldPattern),
    Rest(Span),
}

impl Parser {
    pub(super) fn parse_positional_pattern(
        &mut self,
        path: Vec<Ident>,
        start: u32,
        context: PatternContext,
    ) -> Result<Pattern, Diagnostic> {
        self.with_node(SyntaxKind::PatternList, |parser| {
            parser.bump();
            let diagnostic_start = parser.diagnostics.len();
            let parsed = parser.parse_delimited_items(ListKind::PatternElements, |parser| {
                parser.parse_pattern_item(context)
            });
            if let Some(diagnostic) = parser.diagnostics.get(diagnostic_start) {
                return Err(diagnostic.clone());
            }
            let (elements, rest) = pattern_items(parsed.items)?;
            Ok(Pattern::Positional {
                path,
                elements,
                rest,
                span: Span::new(start, parsed.end),
            })
        })
    }

    pub(super) fn parse_tuple_pattern(
        &mut self,
        context: PatternContext,
    ) -> Result<Pattern, Diagnostic> {
        self.with_node(SyntaxKind::PatternList, |parser| {
            let open = parser.bump();
            let diagnostic_start = parser.diagnostics.len();
            let parsed = parser.parse_delimited_items(ListKind::PatternElements, |parser| {
                parser.parse_pattern_item(context)
            });
            if let Some(diagnostic) = parser.diagnostics.get(diagnostic_start) {
                return Err(diagnostic.clone());
            }
            let (mut elements, rest) = pattern_items(parsed.items)?;
            let span = Span::new(open.span.start, parsed.end);
            if elements.is_empty() && rest.is_none() {
                Ok(Pattern::Literal {
                    value: LiteralPattern::Unit,
                    span,
                })
            } else if elements.len() == 1 && !parsed.had_comma && rest.is_none() {
                Ok(elements.pop().expect("one parenthesized pattern"))
            } else {
                Ok(Pattern::Tuple {
                    elements,
                    rest,
                    span,
                })
            }
        })
    }

    fn parse_pattern_item(&mut self, context: PatternContext) -> Result<PatternItem, Diagnostic> {
        if matches!(self.peek().kind, TokenKind::DotDot) {
            Ok(PatternItem::Rest(self.bump().span))
        } else {
            self.parse_pattern(context).map(PatternItem::Value)
        }
    }

    pub(super) fn parse_named_pattern(
        &mut self,
        path: Vec<Ident>,
        start: u32,
        context: PatternContext,
    ) -> Result<Pattern, Diagnostic> {
        self.with_node(SyntaxKind::PatternList, |parser| {
            parser.bump();
            let diagnostic_start = parser.diagnostics.len();
            let parsed = parser.parse_delimited_items(ListKind::NamedPatternElements, |parser| {
                parser.parse_named_pattern_item(context)
            });
            if let Some(diagnostic) = parser.diagnostics.get(diagnostic_start) {
                return Err(diagnostic.clone());
            }
            let mut fields = Vec::new();
            let mut rest = None;
            for item in parsed.items {
                if rest.is_some() {
                    let span = match &item {
                        NamedItem::Field(field) => field.field.span,
                        NamedItem::Rest(span) => *span,
                    };
                    return Err(Diagnostic::at(
                        span,
                        "`..` must be the last element in a field pattern",
                    ));
                }
                match item {
                    NamedItem::Field(field) => fields.push(field),
                    NamedItem::Rest(span) => rest = Some(span),
                }
            }
            Ok(Pattern::Named {
                path,
                fields,
                rest,
                span: Span::new(start, parsed.end),
            })
        })
    }

    fn parse_named_pattern_item(
        &mut self,
        context: PatternContext,
    ) -> Result<NamedItem, Diagnostic> {
        if matches!(self.peek().kind, TokenKind::DotDot) {
            return Ok(NamedItem::Rest(self.bump().span));
        }
        let start = self.peek().span.start;
        let explicit_binding =
            context == PatternContext::Match && matches!(self.peek().kind, TokenKind::Val);
        let field = if explicit_binding {
            self.bump();
            self.parse_explicit_pattern_binding()?
        } else {
            self.expect_ident("field name")?
        };
        if field.text == "_" {
            return Err(Diagnostic::at(
                field.span,
                "`_` is not allowed in a field pattern",
            ));
        }
        let subpattern = if explicit_binding {
            Pattern::Binding(field.clone())
        } else if matches!(self.peek().kind, TokenKind::Colon) {
            self.bump();
            self.parse_pattern(context)?
        } else if field.text == "Unit" {
            Pattern::Literal {
                value: LiteralPattern::Unit,
                span: field.span,
            }
        } else {
            match context {
                PatternContext::Binding => Pattern::Binding(field.clone()),
                PatternContext::Match => Pattern::BareVariant(field.clone()),
            }
        };
        Ok(NamedItem::Field(FieldPattern {
            span: Span::new(start, pattern_span(&subpattern).end),
            field,
            subpattern: Box::new(subpattern),
        }))
    }
}

fn pattern_items(items: Vec<PatternItem>) -> Result<(Vec<Pattern>, Option<Span>), Diagnostic> {
    let mut elements = Vec::new();
    let mut rest = None;
    for item in items {
        match item {
            PatternItem::Value(value) => elements.push(value),
            PatternItem::Rest(span) => {
                if rest.replace(span).is_some() {
                    return Err(Diagnostic::at(
                        span,
                        "`..` may appear at most once in a pattern",
                    ));
                }
            }
        }
    }
    Ok((elements, rest))
}
