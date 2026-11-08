//! Pattern parsing (spec 4.6 and chapter 5).
//!
//! Pattern positions — destructuring `val` declarations, `for` bindings,
//! lambda parameters, and `when` arm conditions — are pure pattern syntax:
//! `..` here is always the rest marker, never the range operator (the spec
//! 4.6 disambiguation rule).
//! Enum variant patterns and struct patterns share their shapes
//! (`Path(...)`, `Path { ... }`, tuple patterns); HIR resolves which is
//! which. Binding positions accept bare names; match positions require
//! `val name` for bindings and keep bare variant names distinct in AST.
//! A dotted path without an
//! argument list (`E.V`) is a unit variant pattern, encoded as a positional
//! pattern with no elements.

use scoop_ast::{Diagnostic, FieldPattern, Ident, LiteralPattern, Pattern, Span};

use crate::lexer::TokenKind;
use crate::parser::Parser;

mod lists;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum PatternContext {
    Binding,
    Match,
}

/// Every pattern carries a span; `Binding` uses its identifier's.
pub(crate) fn pattern_span(pattern: &Pattern) -> Span {
    match pattern {
        Pattern::Binding(ident) | Pattern::BareVariant(ident) => ident.span,
        Pattern::Wildcard { span }
        | Pattern::Literal { span, .. }
        | Pattern::Positional { span, .. }
        | Pattern::Named { span, .. }
        | Pattern::Tuple { span, .. } => *span,
    }
}

impl Parser {
    pub(crate) fn parse_pattern(&mut self, context: PatternContext) -> Result<Pattern, Diagnostic> {
        self.with_node(scoop_syntax::SyntaxKind::Pattern, |parser| {
            parser.parse_pattern_inner(context)
        })
    }

    fn parse_pattern_inner(&mut self, context: PatternContext) -> Result<Pattern, Diagnostic> {
        let token = self.peek().clone();
        match token.kind {
            TokenKind::Val if context == PatternContext::Match => {
                self.bump();
                self.parse_explicit_pattern_binding().map(Pattern::Binding)
            }
            TokenKind::Minus => self.parse_negated_numeric_literal_pattern(token),
            TokenKind::Float(lexeme) => {
                self.pos += 1;
                Ok(Pattern::Literal {
                    value: LiteralPattern::Float {
                        value: lexeme,
                        negated: false,
                    },
                    span: token.span,
                })
            }
            TokenKind::Int(lexeme) => {
                self.pos += 1;
                Ok(Pattern::Literal {
                    value: LiteralPattern::Integer {
                        value: lexeme.with_span(token.span),
                        negated: false,
                    },
                    span: token.span,
                })
            }
            TokenKind::Char(value) => {
                self.pos += 1;
                Ok(Pattern::Literal {
                    value: LiteralPattern::Char(value),
                    span: token.span,
                })
            }
            TokenKind::Str(value) => {
                self.pos += 1;
                Ok(Pattern::Literal {
                    value: LiteralPattern::String(value),
                    span: token.span,
                })
            }
            TokenKind::True => {
                self.pos += 1;
                Ok(Pattern::Literal {
                    value: LiteralPattern::Boolean(true),
                    span: token.span,
                })
            }
            TokenKind::False => {
                self.pos += 1;
                Ok(Pattern::Literal {
                    value: LiteralPattern::Boolean(false),
                    span: token.span,
                })
            }
            TokenKind::LParen => self.parse_tuple_pattern(context),
            TokenKind::LBrace => self.parse_named_pattern(Vec::new(), token.span.start, context),
            TokenKind::Ident(text) => self.parse_ident_pattern(text, token.span, false, context),
            TokenKind::EscapedIdent(text) => {
                self.parse_ident_pattern(text, token.span, true, context)
            }
            _ => self.unexpected("pattern"),
        }
    }

    fn parse_explicit_pattern_binding(&mut self) -> Result<Ident, Diagnostic> {
        if matches!(&self.peek().kind, TokenKind::Ident(name) if name == "_") {
            return Err(Diagnostic::at(
                self.peek().span,
                "use `_` without `val` to ignore a matched value",
            ));
        }
        self.expect_ident("binding name after `val`")
    }

    /// Unary minus is part of literal-pattern syntax only when its operand is
    /// one numeric token modulo parenthesized disambiguation. Parentheses do
    /// not survive in AST, but their closing delimiter belongs to the unary
    /// expression's source span.
    fn parse_negated_numeric_literal_pattern(
        &mut self,
        prefix: crate::lexer::Token,
    ) -> Result<Pattern, Diagnostic> {
        self.pos += 1;
        let (literal, end) = self.parse_numeric_literal_pattern_operand()?;
        let span = Span::new(prefix.span.start, end);
        Ok(Pattern::Literal {
            value: literal,
            span,
        })
    }

    fn parse_numeric_literal_pattern_operand(
        &mut self,
    ) -> Result<(LiteralPattern, u32), Diagnostic> {
        let token = self.peek().clone();
        match token.kind {
            TokenKind::Int(lexeme) => {
                self.pos += 1;
                Ok((
                    LiteralPattern::Integer {
                        value: lexeme.with_span(token.span),
                        negated: true,
                    },
                    token.span.end,
                ))
            }
            TokenKind::Float(lexeme) => {
                self.pos += 1;
                Ok((
                    LiteralPattern::Float {
                        value: lexeme,
                        negated: true,
                    },
                    token.span.end,
                ))
            }
            TokenKind::LParen => {
                self.pos += 1;
                let (literal, _) = self.parse_numeric_literal_pattern_operand()?;
                let close = self.expect("`)`", |kind| matches!(kind, TokenKind::RParen))?;
                Ok((literal, close.span.end))
            }
            _ => self.unexpected("numeric literal after unary minus"),
        }
    }

    /// An identifier-started pattern: `_`, the `Unit` literal, a binding
    /// name, or a (possibly dotted) variant/struct path with a positional
    /// or field argument list.
    fn parse_ident_pattern(
        &mut self,
        text: String,
        span: Span,
        escaped: bool,
        context: PatternContext,
    ) -> Result<Pattern, Diagnostic> {
        self.pos += 1;
        if text == "_" && !escaped {
            return Ok(Pattern::Wildcard { span });
        }
        if text == "Unit" {
            return Ok(Pattern::Literal {
                value: LiteralPattern::Unit,
                span,
            });
        }
        let start = span.start;
        let mut path = vec![Ident { text, span }];
        // `E.V` — an enum type prefix plus the variant name. The dot must
        // be followed by an identifier; otherwise the path ends.
        while matches!(self.peek().kind, TokenKind::Dot)
            && matches!(
                self.tokens.get(self.pos + 1).map(|token| &token.kind),
                Some(TokenKind::Ident(_) | TokenKind::EscapedIdent(_))
            )
        {
            self.bump(); // `.`
            path.push(self.expect_ident("variant name")?);
        }
        match self.peek().kind {
            TokenKind::LParen => self.parse_positional_pattern(path, start, context),
            TokenKind::LBrace => self.parse_named_pattern(path, start, context),
            _ if path.len() == 1 => {
                let name = path.into_iter().next().expect("path holds one ident");
                Ok(match context {
                    PatternContext::Binding => Pattern::Binding(name),
                    PatternContext::Match => Pattern::BareVariant(name),
                })
            }
            // `E.V` without an argument list is a unit variant pattern.
            _ => {
                let end = path.last().expect("path is non-empty").span.end;
                Ok(Pattern::Positional {
                    path,
                    elements: Vec::new(),
                    rest: None,
                    span: Span::new(start, end),
                })
            }
        }
    }
}
