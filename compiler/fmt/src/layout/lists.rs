use super::{Document, Gap, Input, K, Region, Style, gap, render, sequence, within};
use std::ops::Range;

pub(super) fn delimited(
    input: &Input,
    gaps: &[Gap],
    nested: &[Region],
    region: &Region,
) -> Document {
    let Region { start, end, style } = *region;
    let open = render::token(input.tokens[start].syntax.text());
    let close = render::token(input.tokens[end - 1].syntax.text());
    if end == start + 2 {
        return open.append(close);
    }
    if style == Style::Lambda {
        return open
            .append(lambda_contents(input, gaps, nested, start + 1..end - 1))
            .append(close)
            .group();
    }
    let block = style == Style::Block;
    let contents = start + 1..end - 1;
    let items = if block {
        Vec::new()
    } else {
        comma_items(input, nested, contents.clone())
    };
    let keep_lines = matches!(style, Style::Arguments | Style::Parameters)
        && (gaps[contents.start].is_line()
            || gaps[contents.end].is_line()
            || has_item_break(gaps, &items));
    let edge = if block || keep_lines {
        Document::hardline()
    } else {
        Document::line_()
    };
    let body = if block {
        sequence(
            input,
            gaps,
            within(nested, contents.start, contents.end),
            contents.start,
            contents.end,
            false,
        )
    } else {
        item_documents(
            input,
            gaps,
            nested,
            &items,
            keep_lines && style == Style::Arguments,
        )
    };
    let tail = if input.tokens[end - 2].is_comment() {
        gap(gaps[end - 1])
    } else {
        edge.clone()
    };
    let result = open
        .append(edge.append(body).nest(4))
        .append(tail)
        .append(close);
    if block || keep_lines {
        result
    } else {
        result.group()
    }
}

fn lambda_contents(
    input: &Input,
    gaps: &[Gap],
    nested: &[Region],
    contents: Range<usize>,
) -> Document {
    let lambda = input.tokens[contents.start - 1]
        .syntax
        .parent()
        .expect("lambda opening brace has a syntax parent");
    let arrow = lambda
        .children()
        .find(|node| node.kind() == K::ParameterList)
        .map(|parameters| input.token_range(&parameters).end - 1)
        .or_else(|| {
            contents
                .clone()
                .find(|&index| !input.tokens[index].is_comment())
                .filter(|&index| input.tokens[index].kind() == K::Arrow)
        });
    let separator = |index: usize| {
        if gaps[index].is_line()
            && (input.tokens[index - 1].is_comment() || input.tokens[index].is_comment())
        {
            gap(gaps[index])
        } else {
            Document::line()
        }
    };
    let mut signature = Document::nil();
    let mut body_start = contents.start;
    if let Some(arrow) = arrow {
        signature = separator(contents.start);
        if contents.start < arrow {
            let items = comma_items(input, nested, contents.start..arrow);
            signature = signature
                .append(item_documents(input, gaps, nested, &items, false))
                .append(separator(arrow));
        }
        signature = signature
            .append(render::token(input.tokens[arrow].syntax.text()))
            .nest(4)
            .group();
        body_start = arrow + 1;
    }

    if body_start == contents.end {
        return signature.append(separator(contents.end));
    }

    let body = sequence(
        input,
        gaps,
        within(nested, body_start, contents.end),
        body_start,
        contents.end,
        false,
    );
    // The signature can fit independently of the body. A hard break between
    // statements keeps the enclosing lambda group from flattening.
    signature
        .append(separator(body_start).append(body).nest(4))
        .append(separator(contents.end))
}

pub(super) fn header(input: &Input, gaps: &[Gap], nested: &[Region], region: &Region) -> Document {
    let Region { start, end, style } = *region;
    let items = comma_items(input, nested, start + 1..end);
    let prefix = render::token(input.tokens[start].syntax.text());
    let body = item_documents(input, gaps, nested, &items, false);
    let starts_on_new_line = gaps[start + 1].is_line();
    let items_have_lines = has_item_break(gaps, &items);

    if style == Style::Supertypes {
        let edge = if starts_on_new_line {
            gap(gaps[start + 1])
        } else if items_have_lines {
            Document::softline()
        } else {
            Document::line()
        };
        let doc = prefix.append(edge.append(body).nest(4));
        return if starts_on_new_line || items_have_lines {
            doc
        } else {
            doc.group()
        };
    }

    let multiline = gaps[start].is_line() || starts_on_new_line || items_have_lines;
    let leading = if gaps[start].is_line() {
        gap(gaps[start])
    } else {
        Document::line()
    };
    let separator = if starts_on_new_line && input.tokens[start + 1].is_comment() {
        gap(gaps[start + 1])
    } else {
        Document::space()
    };
    let doc = leading
        .append(prefix)
        .append(separator.append(body).nest(6))
        .nest(4);
    if multiline { doc } else { doc.group() }
}

fn comma_items(input: &Input, nested: &[Region], contents: Range<usize>) -> Vec<Range<usize>> {
    let mut items = Vec::new();
    let mut start = contents.start;
    let mut index = start;
    let mut cursor = 0;
    while index < contents.end {
        if let Some(child) = nested.get(cursor).filter(|child| child.start == index) {
            index = child.end;
            cursor += 1 + nested[cursor + 1..].partition_point(|r| r.start < child.end);
            continue;
        }
        index += 1;
        if input.tokens[index - 1].kind() == K::Comma && index < contents.end {
            items.push(start..index);
            start = index;
        }
    }
    items.push(start..contents.end);
    items
}

fn has_item_break(gaps: &[Gap], items: &[Range<usize>]) -> bool {
    items.iter().skip(1).any(|item| gaps[item.start].is_line())
}

fn item_documents(
    input: &Input,
    gaps: &[Gap],
    nested: &[Region],
    items: &[Range<usize>],
    keep_groups: bool,
) -> Document {
    let mut pieces = Vec::new();
    for (index, item) in items.iter().enumerate() {
        if index > 0 {
            let separator = if input.tokens[item.start].is_comment()
                || keep_groups && gaps[item.start].is_line()
            {
                gap(gaps[item.start])
            } else if keep_groups {
                Document::softline()
            } else {
                Document::line()
            };
            pieces.push(separator);
        }
        pieces.push(sequence(
            input,
            gaps,
            within(nested, item.start, item.end),
            item.start,
            item.end,
            false,
        ));
    }
    Document::concat(pieces)
}
