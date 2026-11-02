use super::*;

impl Lowerer {
    pub(super) fn apply_query_nominal_type(
        &mut self,
        declaration: hir::SourceNominalId,
        arguments: Vec<hir::TypeId>,
        span: Span,
    ) -> Option<hir::TypeId> {
        let parameters = self
            .nominal_declaration_parameters(declaration)
            .map_err(|error| self.error(span, error.diagnostic("queried nominal declaration")))
            .ok()?;
        if parameters.len() != arguments.len() {
            self.error(
                span,
                format!(
                    "queried type takes {} type argument(s), but {} were supplied",
                    parameters.len(),
                    arguments.len()
                ),
            );
            return None;
        }
        if !self.check_type_argument_kinds(&parameters, &arguments, span, "queried type") {
            return None;
        }
        let intrinsic = self.nominal_intrinsic_kind(declaration);
        if matches!(
            intrinsic,
            Some(hir::IntrinsicTypeKind::FunPtr | hir::IntrinsicTypeKind::GcLeafFunPtr)
        ) {
            let hir::Type::Function(function) = self.types[arguments[0]] else {
                let name = if intrinsic == Some(hir::IntrinsicTypeKind::FunPtr) {
                    "FunPtr"
                } else {
                    "GCLeafFunPtr"
                };
                self.error(
                    span,
                    format!("`{name}` type argument must be an ordinary concrete function type"),
                );
                return None;
            };
            let mode = if intrinsic == Some(hir::IntrinsicTypeKind::FunPtr) {
                hir::CAbiCallMode::NativeSafe
            } else {
                hir::CAbiCallMode::GcLeaf
            };
            if !self.validate_native_function_pointer_signature(function, mode, span) {
                return None;
            }
        }
        self.apply_nominal_type(declaration, arguments)
            .map_err(|error| self.error(span, error.diagnostic("queried type application")))
            .ok()
    }

    pub(super) fn query_pointer_declaration(&self, function: bool) -> hir::SourceNominalId {
        match &self.core {
            CoreLoweringAuthority::Defined => {
                let declaration = if function {
                    self.ffi_fun_ptr
                } else {
                    self.ffi_ptr
                }
                .expect("fixed queries follow core pointer declaration resolution");
                self.nominal_identity(Owner::Struct(declaration))
                    .declaration_id()
            }
            CoreLoweringAuthority::Imported(core) => hir::SourceNominalId::GenericTemplate(
                if function {
                    core.fundamental_types().fun_ptr()
                } else {
                    core.fundamental_types().ptr()
                }
                .persistent(),
            ),
        }
    }

    pub(super) fn query_gc_leaf_pointer_declaration(&self) -> hir::SourceNominalId {
        match &self.core {
            CoreLoweringAuthority::Defined => {
                let declaration = self
                    .ffi_gc_leaf_fun_ptr
                    .expect("fixed queries follow core pointer declaration resolution");
                self.nominal_identity(Owner::Struct(declaration))
                    .declaration_id()
            }
            CoreLoweringAuthority::Imported(core) => hir::SourceNominalId::GenericTemplate(
                core.fundamental_types().gc_leaf_fun_ptr().persistent(),
            ),
        }
    }
}
