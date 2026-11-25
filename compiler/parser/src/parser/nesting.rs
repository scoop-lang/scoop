use crate::lexer::{Token, TokenKind};

/// Depth immediately before a token. Indexing by position also works after
/// speculative parses rewind. Each opening delimiter consumes at least one
/// source byte, so both depths fit the source span's u32 representation.
#[derive(Clone, Copy, Default)]
pub(super) struct TokenNesting {
    pub(super) expression: u32,
    pub(super) braces: u32,
}

impl TokenNesting {
    pub(super) fn index(tokens: &[Token]) -> Vec<Self> {
        let mut depth = Self::default();
        tokens
            .iter()
            .map(|token| {
                let before = depth;
                match token.kind {
                    TokenKind::LBrace => {
                        depth.braces += 1;
                        depth.expression += 1;
                    }
                    TokenKind::RBrace => {
                        depth.braces = depth.braces.saturating_sub(1);
                        depth.expression = depth.expression.saturating_sub(1);
                    }
                    TokenKind::LParen | TokenKind::LBracket | TokenKind::InterpolationStart => {
                        depth.expression += 1;
                    }
                    TokenKind::RParen | TokenKind::RBracket | TokenKind::InterpolationEnd => {
                        depth.expression = depth.expression.saturating_sub(1);
                    }
                    _ => {}
                }
                before
            })
            .collect()
    }
}
