//! Descriptions use the established type graph and ordinary substitution.

use super::*;

mod nominals;

impl Lowerer {
    pub(crate) fn describe_semantic_type(
        &mut self,
        ty: hir::TypeId,
    ) -> Result<hir::SemanticTypeDescription, Diagnostic> {
        use hir::SemanticTypeStructure as Shape;
        let structure = match self.types[ty].clone() {
            hir::Type::Unit => Shape::Unit,
            hir::Type::Any => Shape::Any,
            hir::Type::Tuple(elements) => Shape::Tuple(elements),
            hir::Type::Function(function) => Shape::Function(self.function_types[function].clone()),
            hir::Type::Ptr(pointee) => Shape::Pointer(pointee),
            hir::Type::FunPtr(function) => {
                Shape::NativeFunctionPointer(self.function_types[function].clone())
            }
            hir::Type::GcLeafFunPtr(function) => {
                Shape::GcLeafNativeFunctionPointer(self.function_types[function].clone())
            }
            hir::Type::Param(parameter) => Shape::Parameter(self.query_parameter(parameter)?),
            hir::Type::Companion(parameter) => {
                let host = self.query_parameter(parameter)?;
                let host_ty = self.intern_type(hir::Type::Param(parameter));
                if host.companion.is_none() && !self.assumed_conditional_companion(host_ty) {
                    return Err(unbound_parameter(parameter));
                }
                Shape::Companion { host }
            }
            hir::Type::Integer(_)
            | hir::Type::Boolean
            | hir::Type::String
            | hir::Type::Struct(_)
            | hir::Type::Class(_)
            | hir::Type::Enum(_)
            | hir::Type::Interface(_) => Shape::Nominal(
                self.describe_semantic_nominal(ty)?
                    .expect("a resolved nominal retains its declaration"),
            ),
        };
        let visible_companion = match &structure {
            Shape::Nominal(nominal) => nominal
                .companion
                .filter(|&ty| self.nominal_is_accessible(ty)),
            Shape::Parameter(_) => self.projected_companion_type(ty).ok(),
            _ => None,
        };
        Ok(hir::SemanticTypeDescription {
            visible_companion,
            ty,
            structure,
            name: self.type_name(ty),
            parents: self.direct_nominal_supertypes(ty),
        })
    }

    fn query_parameter(
        &self,
        parameter: hir::TypeParamId,
    ) -> Result<hir::TypeParamDecl, Diagnostic> {
        self.type_params_in_scope
            .iter()
            .find(|decl| decl.id == parameter)
            .cloned()
            .ok_or_else(|| unbound_parameter(parameter))
    }
}

fn unbound_parameter(parameter: hir::TypeParamId) -> Diagnostic {
    Diagnostic::at(
        scoop_source::DiagnosticCode::ICE2502,
        Span::new(0, 0),
        hir::StaticShapeUnboundParameter(parameter).to_string(),
    )
}
