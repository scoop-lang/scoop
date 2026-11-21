use super::{Document, Gap, Input, K, Region, gap, sequence, within};
use std::ops::Range;

pub(super) fn continuation(body: Document) -> Document {
    let inline = Document::space().append(body.clone());
    let folded = Document::line().append(body).nest(4);
    inline.union(folded)
}

pub(super) fn expression(
    input: &Input,
    gaps: &[Gap],
    nested: &[Region],
    start: usize,
    end: usize,
) -> Document {
    let mut pieces = Vec::new();
    let mut selectors = Vec::new();
    let mut cursor = 0;
    let mut index = start;
    let mut item = start;
    while index < end {
        if let Some(child) = nested.get(cursor).filter(|child| child.start == index) {
            index = child.end;
            cursor += 1 + nested[cursor + 1..].partition_point(|region| region.start < child.end);
            continue;
        }
        let token = &input.tokens[index];
        if token.is_navigation()
            && input.tokens[index + 1..end]
                .iter()
                .find(|next| !next.is_comment())
                .is_some_and(|next| matches!(next.kind(), K::Ident | K::EscapedIdent))
        {
            selectors.push(index);
        }
        let last_operator_token = token.parent_kind() == K::BinaryOperator
            && !token.is_range_operator()
            && (index + 1 == end || token.operator() != input.tokens[index + 1].operator());
        index += 1;
        if last_operator_token {
            let part = chain(input, gaps, nested, item..index, &selectors);
            pieces.push(if item == start {
                part
            } else {
                Document::line().append(part).nest(4)
            });
            selectors.clear();
            item = index;
        }
    }
    let part = chain(input, gaps, nested, item..end, &selectors);
    if item == start {
        return part;
    }
    pieces.push(Document::line().append(part).nest(4));
    Document::concat(pieces).group()
}

fn chain(
    input: &Input,
    gaps: &[Gap],
    nested: &[Region],
    range: Range<usize>,
    selectors: &[usize],
) -> Document {
    let item = |start, end| sequence(input, gaps, within(nested, start, end), start, end, false);
    let Some(&first_selector) = selectors.first() else {
        return item(range.start, range.end);
    };
    let head = item(range.start, first_selector);
    let tail = selectors.iter().enumerate().map(|(index, &start)| {
        let end = selectors.get(index + 1).copied().unwrap_or(range.end);
        let leading = if gaps[start].is_line() {
            gap(gaps[start])
        } else if input.tokens[start + 1].newlines() > 0 {
            Document::hardline()
        } else if gaps[start] == Gap::Space {
            Document::line()
        } else {
            Document::line_()
        };
        leading.append(item(start, end))
    });
    head.append(Document::concat(tail).nest(4)).group()
}
