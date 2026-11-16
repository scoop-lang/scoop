use super::*;

impl Lowerer {
    pub(super) fn bare_variant_pattern(
        &mut self,
        name: &ast::Ident,
        application: hir::EnumVariantApplication,
        ctx: PatternCtx,
    ) -> Option<hir::Pattern> {
        let variant = self.enum_variant(application);
        let count = variant.fields.len();
        if count != 0 {
            self.error_with_note(
                scoop_source::DiagnosticCode::E0906,
                name.span,
                format!(
                    "variant `{}` of `{}` has {count} field(s)",
                    name.text, variant.owner_name
                ),
                scoop_ast::DiagnosticNote::without_location(
                    scoop_source::DiagnosticCode::H0901,
                    format!("use `{}(...)` to match it", name.text),
                ),
            );
            return None;
        }
        if !ctx.in_when {
            self.error(
                scoop_source::DiagnosticCode::E0902,
                name.span,
                "refutable patterns are only allowed in `when`".into(),
            );
            return None;
        }
        Some(hir::Pattern::Variant {
            application,
            fields: Vec::new(),
        })
    }

    pub(super) fn lower_variant_positional_pattern(
        &mut self,
        application: hir::EnumVariantApplication,
        elements: &[ast::Pattern],
        rest: Option<Span>,
        span: Span,
        ctx: PatternCtx,
    ) -> Option<hir::Pattern> {
        if !ctx.in_when {
            self.error(
                scoop_source::DiagnosticCode::E0902,
                span,
                "refutable patterns are only allowed in `when`".into(),
            );
            return None;
        }
        let variant = self.enum_variant(application);
        let description = variant_owner(&variant.owner_name, &variant.name);
        if variant.style == VariantStyle::Named {
            self.error_with_note(
                scoop_source::DiagnosticCode::E0910,
                span,
                format!("{description} has named fields"),
                scoop_ast::DiagnosticNote::without_location(
                    scoop_source::DiagnosticCode::H0901,
                    "use a named field pattern".to_string(),
                ),
            );
            return None;
        }
        let types = variant
            .fields
            .into_iter()
            .map(|(_, ty)| ty)
            .collect::<Vec<_>>();
        let fields =
            self.lower_positional_pattern(elements, rest, &types, &description, span, ctx)?;
        Some(hir::Pattern::Variant {
            application,
            fields,
        })
    }

    pub(super) fn lower_variant_named_pattern(
        &mut self,
        application: hir::EnumVariantApplication,
        fields: &[ast::FieldPattern],
        rest: Option<Span>,
        span: Span,
        ctx: PatternCtx,
    ) -> Option<hir::Pattern> {
        if !ctx.in_when {
            self.error(
                scoop_source::DiagnosticCode::E0902,
                span,
                "refutable patterns are only allowed in `when`".into(),
            );
            return None;
        }
        let variant = self.enum_variant(application);
        let description = variant_owner(&variant.owner_name, &variant.name);
        if matches!(variant.style, VariantStyle::Unit | VariantStyle::Positional) {
            self.error_with_note(
                scoop_source::DiagnosticCode::E0910,
                span,
                format!("{description} has no named fields"),
                scoop_ast::DiagnosticNote::without_location(
                    scoop_source::DiagnosticCode::H0901,
                    "use a positional pattern".to_string(),
                ),
            );
            return None;
        }
        let fields =
            self.lower_named_fields(fields, rest, &variant.fields, &description, span, ctx)?;
        Some(hir::Pattern::Variant {
            application,
            fields,
        })
    }
}
