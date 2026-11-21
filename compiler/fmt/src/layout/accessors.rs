use super::{Input, K, Region, Style};
use scoop_syntax::SyntaxElement;

pub(super) fn regions(input: &Input, start: usize, end: usize, regions: &mut Vec<Region>) {
    let property = input.tokens[start]
        .syntax
        .parent_ancestors()
        .find(|node| node.kind() == K::PropertyDecl)
        .expect("property starts inside its syntax node");
    let parts: Vec<_> = property
        .children_with_tokens()
        .filter(|part| !part.kind().is_trivia())
        .collect();
    let mut accessors = Vec::new();
    for (index, part) in parts.iter().enumerate() {
        // The property name and identifiers in its initializer are in child nodes.
        let Some(keyword) = part
            .as_token()
            .filter(|token| token.kind() == K::Ident && matches!(token.text(), "get" | "set"))
        else {
            continue;
        };
        let prefix = parts[..index]
            .iter()
            .rposition(|part| !is_accessor_prefix(part))
            .map_or(0, |index| index + 1);
        let offset = parts[prefix].text_range().start();
        let mut begin = input
            .tokens
            .partition_point(|token| token.syntax.text_range().start() < offset);
        while begin > start
            && input.tokens[begin - 1].is_comment()
            && input.tokens[begin - 1].newlines() > 0
        {
            begin -= 1;
        }
        let simple = input.tokens[start].kind() == K::KwVal
            && keyword.text() == "get"
            && input.tokens[begin].syntax == *keyword
            && parts
                .get(index + 3)
                .is_some_and(|part| part.kind() == K::Equal)
            && parts[..index].iter().all(|part| part.kind() != K::Equal);
        accessors.push((begin, simple));
    }

    let mut accessor_end = end;
    for &(start, simple) in accessors.iter().rev() {
        regions.push(Region {
            start,
            end: accessor_end,
            style: if simple && accessors.len() == 1 {
                Style::InlineGetter
            } else {
                Style::Accessor
            },
        });
        accessor_end = start;
    }
}

fn is_accessor_prefix(part: &SyntaxElement) -> bool {
    part.kind() == K::AnnotationList
        || part.as_token().is_some_and(|token| {
            token.kind() == K::Ident
                && matches!(
                    token.text(),
                    "inline" | "public" | "private" | "protected" | "internal"
                )
        })
}
