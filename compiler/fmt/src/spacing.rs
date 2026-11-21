use crate::syntax::{Input, Token};
use scoop_syntax::SyntaxKind as K;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Gap {
    None,
    Space,
    SoftLine,
    Line,
    Blank,
}

impl Gap {
    pub fn is_line(self) -> bool {
        matches!(self, Self::Line | Self::Blank)
    }
}

pub(crate) fn gaps(input: &Input) -> Vec<Gap> {
    let mut gaps = vec![Gap::None; input.tokens.len()];
    for (index, gap) in gaps.iter_mut().enumerate().skip(1) {
        *gap = between(&input.tokens[index - 1], &input.tokens[index]);
    }
    for &(kind, start, end, parent) in &input.nodes {
        let separation = match kind {
            K::Declaration if parent == Some(K::SourceFile) => Some(Gap::Blank),
            K::PackageHeader => Some(Gap::Blank),
            K::ImportHeader => Some(Gap::Line),
            K::WhenArm if separates_when_blocks(input, start) => Some(Gap::Blank),
            K::WhenArm | K::Member => Some(Gap::Line),
            K::Statement if matches!(parent, Some(K::Block | K::LambdaExpr)) => Some(Gap::Line),
            _ => None,
        };
        if let Some(separation) = separation {
            let mut leading = start;
            while leading > 0
                && input.tokens[leading - 1].is_comment()
                && (leading == 1 || input.tokens[leading - 1].newlines() > 0)
            {
                leading -= 1;
            }
            if leading > 0 {
                gaps[leading] = if gaps[leading] == Gap::Blank {
                    Gap::Blank
                } else {
                    separation
                };
            }
        }
        if kind == K::PackageHeader && end < gaps.len() {
            gaps[end] = Gap::Blank;
        }
        if kind == K::SingleControlBody {
            if is_else_if(input, start, parent) {
                gaps[start] = Gap::Space;
            }
            if let Some(next) = input.tokens.get(end) {
                match next.kind() {
                    K::KwElse if !input.tokens[end - 1].is_comment() => gaps[end] = Gap::SoftLine,
                    K::KwWhile => gaps[end] = Gap::Line,
                    _ => {}
                }
            }
        }
        if kind == K::AnnotationList
            && matches!(parent, Some(K::Declaration | K::Member | K::Statement))
        {
            declaration_annotations(input, &mut gaps, start, end);
        }
    }
    if !gaps.is_empty() {
        gaps[0] = Gap::None;
    }
    gaps
}

fn separates_when_blocks(input: &Input, start: usize) -> bool {
    let arm = input.tokens[start]
        .syntax
        .parent_ancestors()
        .find(|node| node.kind() == K::WhenArm)
        .expect("when arm starts with its condition");
    let has_body = |node: &scoop_syntax::SyntaxNode| {
        node.children()
            .any(|child| child.kind() == K::Block && input.token_range(&child).len() > 2)
    };
    arm.prev_sibling()
        .filter(|previous| previous.kind() == K::WhenArm)
        .is_some_and(|previous| has_body(&previous) || has_body(&arm))
}

pub(crate) fn is_else_if(input: &Input, start: usize, parent: Option<K>) -> bool {
    parent == Some(K::IfExpr)
        && input.tokens[start].kind() == K::KwIf
        && start > 0
        && input.tokens[start - 1].kind() == K::KwElse
}

fn declaration_annotations(input: &Input, gaps: &mut [Gap], start: usize, end: usize) {
    let list = input.tokens[start]
        .syntax
        .parent_ancestors()
        .find(|node| node.kind() == K::AnnotationList)
        .expect("annotation list starts with an annotation");
    let mut multiple = false;
    for annotation in list.children().filter(|node| node.kind() == K::Annotation) {
        let range = input.token_range(&annotation);
        if annotation
            .children()
            .any(|child| child.kind() == K::ArgumentList && !child.text_range().is_empty())
        {
            if range.start > start && gaps[range.start] == Gap::Space {
                gaps[range.start] = Gap::Line;
            }
            if gaps.get(range.end) == Some(&Gap::Space) {
                gaps[range.end] = Gap::Line;
            }
        }
        multiple |= range.start > start;
    }
    if multiple && gaps.get(end) == Some(&Gap::Space) {
        gaps[end] = Gap::Line;
    }
}

fn between(left: &Token, right: &Token) -> Gap {
    use Gap::*;
    let l = left.kind();
    let r = right.kind();
    if left.is_comment() || right.is_comment() {
        return if matches!(l, K::LineComment | K::DocLineComment | K::Shebang)
            || right.newlines() > 0
        {
            if right.newlines() > 1 { Blank } else { Line }
        } else {
            Space
        };
    }
    if matches!(l, K::FStringStart | K::FStringText | K::InterpolationStart)
        || matches!(
            r,
            K::FStringText | K::FStringEnd | K::InterpolationStart | K::InterpolationEnd
        )
    {
        return None;
    }
    let lp = left.parent_kind();
    let rp = right.parent_kind();
    if left.is_range_operator() || right.is_range_operator() {
        return None;
    }
    if l == K::Colon && lp == K::Annotation || r == K::Colon && rp == K::Annotation {
        return None;
    }
    if left.is_navigation() && matches!(r, K::Ident | K::EscapedIdent) && rp == K::Expression {
        return None;
    }
    if lp == K::BinaryOperator
        && rp == K::BinaryOperator
        && left.syntax.parent() == right.syntax.parent()
    {
        return None;
    }
    if lp == K::BinaryOperator || rp == K::BinaryOperator {
        return Space;
    }
    if r == K::Colon && rp == K::SupertypeList {
        return Space;
    }
    if r == K::LBrace && joins_header(right)
        || l == K::RBrace && joins_clause(left, right)
        || l == K::Ident && r == K::LParen && lp == K::TryExpr && rp == K::TryExpr
    {
        return Space;
    }
    // Other newlines can terminate return operands, infix calls, and when-arm bodies.
    // Delimited-list whitespace is relaxed by the layout layer at its own separators.
    if right.newlines() > 0 {
        return if right.newlines() > 1 { Blank } else { Line };
    }
    if rp == K::PostfixOperator || lp == K::PrefixOperator {
        return if would_merge(left, right) {
            Space
        } else {
            None
        };
    }
    if lp == K::PostfixOperator && rp == K::PostfixOperator {
        return None;
    }
    if l == K::Colon
        || r == K::Colon
            && matches!(
                rp,
                K::SupertypeList | K::TypeParameter | K::TypeConstraint | K::SecondaryConstructor
            )
    {
        return Space;
    }
    if matches!(
        r,
        K::Comma | K::Semicolon | K::Colon | K::RParen | K::RBracket | K::Question
    ) || matches!(
        l,
        K::Dot | K::QuestionDot | K::DoubleColon | K::At | K::LParen | K::LBracket
    ) || matches!(r, K::Dot | K::QuestionDot | K::DoubleColon)
    {
        return None;
    }
    if r == K::Less && matches!(rp, K::TypeArgumentList | K::TypeParameterList) {
        return if l == K::KwFun { Space } else { None };
    }
    if l == K::Less && matches!(lp, K::TypeArgumentList | K::TypeParameterList)
        || r == K::Greater && matches!(rp, K::TypeArgumentList | K::TypeParameterList)
    {
        return None;
    }
    if r == K::LBracket && rp == K::IndexArguments {
        return None;
    }
    if r == K::LParen {
        return if matches!(
            l,
            K::KwIf | K::KwWhile | K::KwFor | K::KwWhen | K::KwSuspend
        ) {
            Space
        } else if matches!(
            rp,
            K::ArgumentList | K::ParameterList | K::PatternList | K::ContextParameters
        ) || right.before.is_empty()
        {
            None
        } else {
            Space
        };
    }
    if r == K::At
        && rp != K::Annotation
        && matches!(
            l,
            K::KwReturn | K::KwBreak | K::KwContinue | K::Ident | K::EscapedIdent
        )
    {
        return None;
    }
    if l == K::Star && lp != K::BinaryOperator {
        return None;
    }
    if l == K::Semicolon {
        return Line;
    }
    Space
}

fn joins_header(brace: &Token) -> bool {
    match brace.parent_kind() {
        K::ClassBody | K::StructBody | K::InterfaceBody | K::EnumDecl | K::WhenExpr => true,
        K::Block => brace
            .syntax
            .parent()
            .and_then(|block| block.parent())
            .is_some_and(|owner| match owner.kind() {
                K::FunctionDecl
                | K::AnonymousFunctionExpr
                | K::SecondaryConstructor
                | K::PropertyDecl
                | K::InitBlock
                | K::ReleaseBlock
                | K::IfExpr
                | K::TryExpr
                | K::WhileStmt
                | K::ForStmt
                | K::WhenArm => true,
                K::Statement => owner
                    .children_with_tokens()
                    .any(|item| item.kind() == K::KwDo),
                _ => false,
            }),
        _ => false,
    }
}

fn joins_clause(brace: &Token, keyword: &Token) -> bool {
    brace
        .syntax
        .parent()
        .filter(|body| body.kind() == K::Block)
        .and_then(|body| body.parent())
        .is_some_and(|owner| {
            matches!(
                (owner.kind(), keyword.kind()),
                (K::IfExpr, K::KwElse) | (K::TryExpr, K::Ident) | (K::Statement, K::KwWhile)
            ) && keyword.syntax.parent().as_ref() == Some(&owner)
        })
}

fn would_merge(left: &Token, right: &Token) -> bool {
    matches!(
        (left.kind(), right.kind()),
        (K::Plus, K::Plus | K::PlusPlus | K::Equal)
            | (K::Minus, K::Minus | K::MinusMinus | K::Equal | K::Greater)
            | (K::Bang, K::Equal | K::KwIn)
    )
}
