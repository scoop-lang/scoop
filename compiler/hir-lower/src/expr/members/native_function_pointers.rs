use super::*;

impl Lowerer {
    pub(in crate::expr) fn normalize_native_function_pointer_conversion(
        &self,
        kind: hir::NativeFunctionPointerIntrinsic,
        operand: hir::Expr,
        result_type: hir::TypeId,
        span: Span,
    ) -> hir::Expr {
        let target_mode = match kind {
            hir::NativeFunctionPointerIntrinsic::AsFunPtr => hir::CAbiCallMode::NativeSafe,
            hir::NativeFunctionPointerIntrinsic::AsGcLeaf => hir::CAbiCallMode::GcLeaf,
        };
        hir::Expr {
            kind: hir::ExprKind::NativeFunctionPointerCast {
                operand: Box::new(operand),
                target_mode,
            },
            ty: result_type,
            span,
            origin: self.expression_origin(span),
        }
    }
}
