use scoop_ast::{ClassDecl, FunctionDecl, VisibilitySyntax};
use scoop_source::{SourceEdit, Span};
use scoop_syntax::{SyntaxKind as K, SyntaxNode};

use crate::SourceInput;

use super::super::extract_local::{contains, syntax::fresh_name};
use super::super::inline_local::{removal_span, updated_span};
use super::super::move_declaration::attached_comments;

pub(super) struct MemberEdits {
    pub edits: Vec<SourceEdit>,
    moved: Vec<(Span, u32)>,
    expanded_class: Option<(Span, u32)>,
}

impl MemberEdits {
    pub fn map(&self, span: Span) -> Option<Span> {
        if let Some(&(old, new)) = self.moved.iter().find(|&&(old, _)| contains(old, span)) {
            return Some(Span::new(
                new + (span.start - old.start),
                new + (span.end - old.start),
            ));
        }
        let mut mapped = updated_span(span, &self.edits)?;
        if let Some((class, body_size)) = self.expanded_class
            && span == class
        {
            // A primary constructor's source origin can cover the entire class.
            mapped.end = mapped.end.checked_add(body_size)?;
        }
        Some(mapped)
    }
}

pub(super) fn extract_interface(
    input: &SourceInput,
    root: &SyntaxNode,
    owner: &ClassDecl,
    visibility: &str,
    methods: &[&FunctionDecl],
) -> Option<MemberEdits> {
    let name = fresh_name(root, "ExtractedInterface")?;
    let newline = newline(input);
    let text = input.text.as_str();
    let mut interface = format!("{visibility}interface {name} {{{newline}");
    let mut edits = vec![add_parent(input, owner, &name)];
    for method in methods {
        let mut token = root
            .token_at_offset(method.name.span.start.into())
            .left_biased()?;
        while token.kind() != K::KwFun {
            if u32::from(token.text_range().start()) < method.span.start {
                return None;
            }
            token = token.prev_token()?;
        }
        let keyword = u32::from(token.text_range().start());
        let mut explicit_final = false;
        while let Some(previous) = token.prev_token() {
            if u32::from(previous.text_range().start()) < method.span.start {
                break;
            }
            explicit_final |= previous.kind() == K::Ident && previous.text() == "final";
            token = previous;
        }
        interface.push_str("    ");
        if let VisibilitySyntax::Explicit { span, .. } = method.visibility {
            interface.push_str(text.get(span.start as usize..span.end as usize)?);
            interface.push(' ');
        }
        interface.push_str(text.get(keyword as usize..method.signature.span.end as usize)?);
        interface.push_str(newline);
        edits.push(insert(
            input,
            keyword,
            if explicit_final {
                "override "
            } else {
                "final override "
            }
            .to_owned(),
        ));
    }
    interface.push_str(&format!("}}{newline}{newline}"));
    edits.push(insert(
        input,
        attached_comments(input, owner.span).start,
        interface,
    ));
    edits.sort_unstable_by_key(|edit| edit.span.start);
    Some(MemberEdits {
        edits,
        moved: Vec::new(),
        expanded_class: None,
    })
}

pub(super) fn extract_superclass(
    input: &SourceInput,
    root: &SyntaxNode,
    owner: &ClassDecl,
    visibility: &str,
    methods: &[&FunctionDecl],
) -> Option<MemberEdits> {
    let name = fresh_name(root, "ExtractedBase")?;
    let newline = newline(input);
    let position = attached_comments(input, owner.span).start;
    insert_methods(
        input,
        position,
        format!("{visibility}open class {name}() {{{newline}    "),
        &format!("{newline}    "),
        &format!("{newline}}}{newline}{newline}"),
        methods,
        vec![add_parent(input, owner, &format!("{name}()"))],
    )
}

pub(super) fn move_members(
    input: &SourceInput,
    root: &SyntaxNode,
    target: &ClassDecl,
    methods: &[&FunctionDecl],
) -> Option<MemberEdits> {
    let decl = root
        .token_at_offset(target.name.span.start.into())
        .right_biased()?
        .parent_ancestors()
        .find(|node| node.kind() == K::ClassDecl)?;
    let text = input.text.as_str();
    let newline = newline(input);
    let line = text
        .get(..target.span.start as usize)?
        .rsplit(['\r', '\n'])
        .next()?;
    let indent = &line[..line.len() - line.trim_start_matches([' ', '\t']).len()];
    let between = format!("{newline}{indent}    ");
    let close_indent = format!("{newline}{indent}");
    let body = decl.children().find(|node| node.kind() == K::ClassBody);
    let adds_body = body.is_none();
    let (position, prefix, suffix) = if let Some(body) = body {
        let close = body
            .children_with_tokens()
            .filter_map(|element| element.into_token())
            .find(|token| token.kind() == K::RBrace)?;
        let close = u32::from(close.text_range().start());
        let line = text.get(..close as usize)?.rsplit(['\r', '\n']).next()?;
        if line.trim().is_empty() {
            (
                close.checked_sub(u32::try_from(line.len()).ok()?)?,
                format!("{indent}    "),
                newline.to_owned(),
            )
        } else {
            (close, between.clone(), close_indent)
        }
    } else {
        (
            target.header.span.end,
            format!(" {{{between}"),
            format!("{close_indent}}}"),
        )
    };
    let mut plan = insert_methods(
        input,
        position,
        prefix,
        &between,
        &suffix,
        methods,
        Vec::new(),
    )?;
    if adds_body {
        let insertion = plan
            .edits
            .iter()
            .find(|edit| edit.span == Span::new(position, position))?;
        plan.expanded_class = Some((
            target.span,
            u32::try_from(insertion.replacement.len()).ok()?,
        ));
    }
    Some(plan)
}

fn insert_methods(
    input: &SourceInput,
    position: u32,
    mut contents: String,
    between: &str,
    suffix: &str,
    methods: &[&FunctionDecl],
    mut edits: Vec<SourceEdit>,
) -> Option<MemberEdits> {
    let text = input.text.as_str();
    let mut moved = Vec::new();
    for (index, method) in methods.iter().enumerate() {
        if index != 0 {
            contents.push_str(between);
        }
        let original = attached_comments(input, method.span);
        moved.push((original, u32::try_from(contents.len()).ok()?));
        contents.push_str(text.get(original.start as usize..original.end as usize)?);
        edits.push(SourceEdit {
            source: input.version,
            span: removal_span(text, original),
            replacement: String::new(),
        });
    }
    contents.push_str(suffix);
    edits.sort_unstable_by_key(|edit| (edit.span.start, edit.span.end));
    let slot = edits.partition_point(|edit| edit.span.end <= position);
    let point = Span::new(position, position);
    let base = updated_span(point, &edits[..slot])?.start;
    for (_, offset) in &mut moved {
        *offset = base.checked_add(*offset)?;
    }
    edits.insert(slot, insert(input, position, contents));
    Some(MemberEdits {
        edits,
        moved,
        expanded_class: None,
    })
}

fn add_parent(input: &SourceInput, owner: &ClassDecl, parent: &str) -> SourceEdit {
    let separator = if owner.supertypes.is_empty() {
        " : "
    } else {
        ", "
    };
    insert(input, owner.header.span.end, format!("{separator}{parent}"))
}

fn insert(input: &SourceInput, position: u32, replacement: String) -> SourceEdit {
    SourceEdit {
        source: input.version,
        span: Span::new(position, position),
        replacement,
    }
}

fn newline(input: &SourceInput) -> &'static str {
    if input.text.as_str().contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    }
}
