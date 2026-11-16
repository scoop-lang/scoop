use super::*;

impl Lowerer {
    pub(crate) fn release_has_field_name(&self, name: &str) -> bool {
        self.current_release.is_some_and(|class| {
            self.classes[class]
                .properties
                .iter()
                .any(|property| self.properties[*property].name == name)
        })
    }

    pub(crate) fn release_field_read(&mut self, name: &ast::Ident) -> Option<hir::Expr> {
        let class = self.current_release?;
        let property = self.classes[class]
            .properties
            .iter()
            .copied()
            .find(|property| self.properties[*property].name == name.text)?;
        let property = self.properties[property].clone();
        let hir::PropertyRepresentation::Stored(hir::StoredProperty {
            backing: hir::PropertyBacking::ClassField { field, .. },
            ..
        }) = &property.representation
        else {
            self.error(scoop_source::DiagnosticCode::E1111, name.span, format!("`release` can only read own backing fields; `{}` has no ordinary backing field", name.text));
            return None;
        };
        Some(hir::Expr {
            kind: hir::ExprKind::ReleaseFieldLoad(hir::ReleaseFieldRef {
                owner: self.class_applications[self.classes[class].self_application].canonical_type,
                field: self.class_field_identity(*field),
            }),
            ty: property.ty,
            span: name.span,
            origin: self.expression_origin(name.span),
        })
    }

    pub(crate) fn reject_release_field_write(&mut self, name: &ast::Ident) -> bool {
        if self.scopes.lookup(&name.text).is_none() && self.release_has_field_name(&name.text) {
            self.error_with_note(
                scoop_source::DiagnosticCode::E1112,
                name.span,
                format!(
                    "release field `{__diagnostic_arg0}` is read-only",
                    __diagnostic_arg0 = name.text
                ),
                scoop_source::DiagnosticNote::without_location(
                    scoop_source::DiagnosticCode::H1501,
                    "copy it to a local value before updating".to_owned(),
                ),
            );
            return true;
        }
        false
    }
}
