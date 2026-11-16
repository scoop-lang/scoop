//! Compiler-known construction of raw FFI pointer values.

use super::*;

impl Lowerer {
    pub(in crate::expr) fn lower_ffi_struct_init(
        &mut self,
        struct_id: hir::StructId,
        call: CallSite<'_>,
        sink: &mut Vec<hir::Statement>,
        expected: Option<TypeId>,
    ) -> Option<hir::Expr> {
        if Some(struct_id) == self.ffi_fun_ptr {
            self.error(
                scoop_source::DiagnosticCode::E0510,
                call.span,
                "intrinsic struct `FunPtr` has no source constructor".to_string(),
            );
            return None;
        }
        if Some(struct_id) == self.ffi_gc_leaf_fun_ptr {
            self.error(
                scoop_source::DiagnosticCode::E0510,
                call.span,
                "intrinsic struct `GCLeafFunPtr` has no source constructor".to_string(),
            );
            return None;
        }
        debug_assert_eq!(Some(struct_id), self.ffi_ptr);
        let parameters = self.structs[struct_id].type_params.clone();
        let diagnostics_before = self.diagnostics.len();
        let expression = self.lower_raw_pointer_init(&parameters, call, sink, expected);
        if self.diagnostics[diagnostics_before..]
            .iter()
            .any(|diagnostic| diagnostic.severity == ast::DiagnosticSeverity::Error)
        {
            return None;
        }
        expression
    }

    pub(in crate::expr) fn lower_raw_pointer_init(
        &mut self,
        parameters: &[hir::TypeParamDecl],
        call: CallSite<'_>,
        sink: &mut Vec<hir::Statement>,
        expected: Option<TypeId>,
    ) -> Option<hir::Expr> {
        let explicit = self.resolve_call_type_args(call.type_args)?;
        if explicit.len() > 1 {
            self.error(
                scoop_source::DiagnosticCode::E0601,
                call.span,
                format!(
                    "`Ptr` expects one explicit type argument, but {} were supplied",
                    explicit.len()
                ),
            );
            return None;
        }
        let contextual_pointee = expected.and_then(|ty| match self.types[ty] {
            Type::Ptr(pointee) => Some(pointee),
            _ => None,
        });
        let pointee = match explicit.as_slice() {
            [ResolvedCallTypeArgument::Explicit { ty, .. }] => *ty,
            [ResolvedCallTypeArgument::Infer { .. }] | [] => {
                let Some(pointee) = contextual_pointee else {
                    self.error_with_note(
                        scoop_source::DiagnosticCode::E0607,
                        call.span,
                        "cannot infer `Ptr` pointee type".to_owned(),
                        scoop_source::DiagnosticNote::without_location(
                            scoop_source::DiagnosticCode::H0502,
                            "supply one explicit type argument".to_owned(),
                        ),
                    );
                    return None;
                };
                pointee
            }
            _ => unreachable!("the explicit Ptr arity was checked"),
        };
        if !self.check_type_argument_kinds(parameters, &[pointee], call.span, "struct `Ptr`") {
            return None;
        }
        if !self.type_contains_param(pointee) && !self.is_gc_free(pointee) {
            let found = self.type_name(pointee);
            self.error(
                scoop_source::DiagnosticCode::E1213,
                call.span,
                format!("`Ptr` pointee must be GC-free, found {found}"),
            );
            return None;
        }

        let mut raw_argument = None;
        for argument in call.args {
            match &argument.name {
                ast::CallArgumentName::Positional | ast::CallArgumentName::TrailingLambda => {}
                ast::CallArgumentName::Named(name) if name.text == "raw" => {}
                ast::CallArgumentName::Named(name) => {
                    self.error(
                        scoop_source::DiagnosticCode::E0702,
                        name.span,
                        format!("`Ptr` has no value parameter named `{}`", name.text),
                    );
                    return None;
                }
            }
            if raw_argument.is_some() {
                self.error(
                    scoop_source::DiagnosticCode::E0703,
                    argument.span,
                    "`Ptr` value parameter `raw` is supplied more than once".to_string(),
                );
                return None;
            }
            if !matches!(argument.spread, ast::SpreadSyntax::Plain) {
                self.error(
                    scoop_source::DiagnosticCode::E0707,
                    argument.span,
                    "spread is not allowed for `Ptr` value parameter `raw`".to_string(),
                );
                return None;
            }
            raw_argument = Some(argument);
        }
        let Some(argument) = raw_argument else {
            self.error(
                scoop_source::DiagnosticCode::E0708,
                call.span,
                "required `Ptr` value parameter `raw` has no argument".to_string(),
            );
            return None;
        };

        self.require_unsafe_operation(call.span, "constructing `Ptr` from a raw integer");
        let ulong = self.integer_type(hir::IntegerKind::UNSIGNED_64);
        let raw = self.lower_expr(&argument.expression, sink, Some(ulong))?;
        if !self.is_subtype(raw.ty, ulong) {
            let found = self.type_name(raw.ty);
            self.error(
                scoop_source::DiagnosticCode::E0501,
                argument.span,
                format!("`Ptr` raw address must be ULong, found {found}"),
            );
            return None;
        }
        let raw = self.adapt_to(raw, ulong);
        if crate::globals::evaluate_hir_integer_constant(&raw, sink)
            .is_some_and(|value| value.raw_bits() == 0)
        {
            self.error(
                scoop_source::DiagnosticCode::E1215,
                argument.span,
                "`Ptr` raw address must be nonzero".to_string(),
            );
            // Keep the candidate applicable. Only the selected entry commits
            // this diagnostic, just like its unsafe-context requirement.
        }
        let ty = self.intern_type(Type::Ptr(pointee));
        Some(hir::Expr {
            kind: ExprKind::PtrFromNonZeroULong(Box::new(raw)),
            ty,
            span: call.span,
            origin: self.expression_origin(call.span),
        })
    }
}
