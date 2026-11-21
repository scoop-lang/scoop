use crate::{
    render::{self, Document},
    spacing::{Gap, is_else_if},
    syntax::Input,
};
use scoop_syntax::SyntaxKind as K;

mod expressions;
mod lists;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Style {
    List,
    Arguments,
    Parameters,
    Supertypes,
    Where,
    Block,
    Lambda,
    Control,
    If,
    IfBody,
    Condition,
    WhenBody,
    Expression,
    Continuation,
}

struct Region {
    start: usize,
    end: usize,
    style: Style,
}

pub(crate) fn document(input: &Input, gaps: &[Gap]) -> Document {
    let regions = regions(input);
    sequence(input, gaps, &regions, 0, input.tokens.len(), false)
}

fn regions(input: &Input) -> Vec<Region> {
    let mut regions = Vec::new();
    let mut stack = Vec::new();
    for (index, token) in input.tokens.iter().enumerate() {
        let kind = token.kind();
        let parent = token.parent_kind();
        let generic = matches!(parent, K::TypeArgumentList | K::TypeParameterList);
        if matches!(
            kind,
            K::LParen | K::LBracket | K::LBrace | K::InterpolationStart
        ) || kind == K::Less && generic
        {
            let style = if kind == K::LBrace {
                match parent {
                    K::LambdaExpr => Style::Lambda,
                    K::CopyUpdateFields | K::PatternList | K::ParameterList => Style::List,
                    _ => Style::Block,
                }
            } else {
                match (kind, parent) {
                    (K::LParen, K::ArgumentList) => Style::Arguments,
                    (K::LParen, K::ParameterList) => Style::Parameters,
                    (K::LParen, K::IfExpr | K::WhenExpr) => Style::Condition,
                    _ => Style::List,
                }
            };
            stack.push((index, style));
        } else if matches!(
            kind,
            K::RParen | K::RBracket | K::RBrace | K::InterpolationEnd
        ) || kind == K::Greater && generic
        {
            let (start, style) = stack
                .pop()
                .expect("successful parse has balanced delimiters");
            regions.push(Region {
                start,
                end: index + 1,
                style,
            });
        }
    }
    for &(kind, start, end, parent) in &input.nodes {
        if let Some(style) = match kind {
            K::SupertypeList => Some(Style::Supertypes),
            K::WhereClause => Some(Style::Where),
            K::IfExpr => Some(Style::If),
            K::Statement if parent == Some(K::WhenArm) => Some(Style::WhenBody),
            K::SingleControlBody if !is_else_if(input, start, parent) => {
                if parent == Some(K::IfExpr) {
                    Some(Style::IfBody)
                } else {
                    Some(Style::Control)
                }
            }
            _ => None,
        } {
            regions.push(Region { start, end, style });
        }
        if kind == K::Expression && start > 0 && input.tokens[start - 1].kind() == K::Equal {
            regions.push(Region {
                start,
                end,
                style: Style::Continuation,
            });
        }
    }
    for &(start, end) in &input.expressions {
        regions.push(Region {
            start,
            end,
            style: Style::Expression,
        });
    }
    regions.sort_by_key(|region| {
        (
            region.start,
            std::cmp::Reverse(region.end),
            match region.style {
                Style::Control | Style::IfBody | Style::WhenBody => 0,
                Style::Continuation => 1,
                Style::If => 2,
                Style::Expression => 3,
                _ => 4,
            },
        )
    });
    regions
}

fn sequence(
    input: &Input,
    gaps: &[Gap],
    regions: &[Region],
    start: usize,
    end: usize,
    leading: bool,
) -> Document {
    let mut docs = Vec::new();
    let mut index = start;
    let mut cursor = 0;
    while index < end {
        if (leading || index != start)
            && !regions.get(cursor).is_some_and(|r| {
                r.start == index
                    && matches!(
                        r.style,
                        Style::Control
                            | Style::IfBody
                            | Style::WhenBody
                            | Style::Continuation
                            | Style::Where
                    )
            })
        {
            docs.push(gap(gaps[index]));
        }
        if let Some(region) = regions.get(cursor).filter(|region| region.start == index) {
            let nested_end = cursor
                + 1
                + regions[cursor + 1..].partition_point(|nested| nested.start < region.end);
            let nested = &regions[cursor + 1..nested_end];
            docs.push(delimited(input, gaps, nested, region));
            index = region.end;
            cursor = nested_end;
        } else {
            docs.push(render::token(input.tokens[index].syntax.text()));
            index += 1;
        }
    }
    Document::concat(docs)
}

fn delimited(input: &Input, gaps: &[Gap], nested: &[Region], region: &Region) -> Document {
    let Region { start, end, style } = *region;
    if style == Style::Continuation {
        return expressions::continuation(sequence(input, gaps, nested, start, end, false));
    }
    if style == Style::Expression {
        return expressions::expression(input, gaps, nested, start, end);
    }
    if matches!(style, Style::Control | Style::IfBody | Style::WhenBody) {
        let leading = if style == Style::Control {
            Document::hardline()
        } else if gaps[start].is_line()
            && (style == Style::WhenBody || input.tokens[start - 1].is_comment())
        {
            gap(gaps[start])
        } else {
            Document::line()
        };
        let body = leading
            .append(sequence(input, gaps, nested, start, end, false))
            .nest(4);
        return if style == Style::WhenBody {
            body.group()
        } else {
            body
        };
    }
    if style == Style::If {
        return sequence(input, gaps, nested, start, end, false).group();
    }
    if style == Style::Condition {
        return condition(input, gaps, nested, start, end);
    }
    if matches!(style, Style::Supertypes | Style::Where) {
        lists::header(input, gaps, nested, region)
    } else {
        lists::delimited(input, gaps, nested, region)
    }
}

fn condition(input: &Input, gaps: &[Gap], nested: &[Region], start: usize, end: usize) -> Document {
    let contents = sequence(
        input,
        gaps,
        within(nested, start + 1, end - 1),
        start + 1,
        end - 1,
        false,
    );
    let contents = if input.tokens[start + 1].is_comment() && gaps[start + 1].is_line() {
        gap(gaps[start + 1]).append(contents).nest(4)
    } else {
        contents
    };
    let tail = if input.tokens[end - 2].is_comment() {
        gap(gaps[end - 1])
    } else {
        Document::line_()
    };
    render::token(input.tokens[start].syntax.text())
        .append(contents)
        .append(tail)
        .append(render::token(input.tokens[end - 1].syntax.text()))
        .group()
}

fn within(regions: &[Region], start: usize, end: usize) -> &[Region] {
    let first = regions.partition_point(|region| region.start < start);
    let last = regions.partition_point(|region| region.start < end);
    &regions[first..last]
}

fn gap(gap: Gap) -> Document {
    match gap {
        Gap::None => Document::nil(),
        Gap::Space => Document::space(),
        Gap::SoftLine => Document::line(),
        Gap::Line => Document::hardline(),
        Gap::Blank => Document::hardline().append(Document::hardline()),
    }
}
