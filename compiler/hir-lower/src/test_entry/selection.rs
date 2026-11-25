use super::*;

impl Lowerer {
    pub(super) fn test_case_selected(
        &mut self,
        arguments: hir::Expr,
        index: usize,
        case: &hir::TestCase,
        sink: &mut Vec<hir::Statement>,
    ) -> Option<hir::Expr> {
        let span = case.method.origin.span;
        let key = self.test_string(index.to_string(), span);
        let ignored = hir::Expr {
            kind: hir::ExprKind::BoolLiteral(case.ignore_reason.is_some()),
            ty: self.boolean,
            span,
            origin: self.expression_origin(span),
        };
        let name = self.test_string(case.display_name.clone(), span);
        let selected = self.test_api_call(
            "isTestSelected",
            vec![arguments, key, ignored, name],
            span,
            sink,
        )?;
        if !self.types_equal(selected.ty, self.boolean) {
            self.error(
                scoop_source::DiagnosticCode::E1403,
                span,
                "test execution requires isTestSelected to return Boolean".into(),
            );
            return None;
        }
        Some(selected)
    }

    pub(super) fn test_string(&self, value: String, span: ast::Span) -> hir::Expr {
        hir::Expr {
            kind: hir::ExprKind::StringLiteral {
                value,
                owner: hir::StringConstantOwner::CurrentDefinition,
            },
            ty: self.string,
            span,
            origin: self.expression_origin(span),
        }
    }
}
