use super::*;
use hir::ImportedCallableSource;

impl Lowerer {
    pub(crate) fn imported_call_argument_mode(
        &mut self,
        candidate: &impl ImportedCallableSource,
        name: &scoop_ast::Ident,
        span: Span,
    ) -> Option<ArgumentMode> {
        let mut argument_mode = ArgumentMode::Mixed;
        let interface = candidate.interface();
        if let scoop_identity::CallableTemplateOrigin::VariantConstructor(variant) =
            interface.declaration()
        {
            let hir::PublicDeclarationOwnerV1::Nominal(owner) = interface.owner() else {
                unreachable!("a dependency variant retains its enum owner")
            };
            let enumeration = self
                .dependencies
                .as_ref()
                .and_then(|dependencies| dependencies.nominal_declaration(owner))
                .expect("the variant owner is in the dependency catalog");
            let hir::NominalSourceShapeV1::Enum(shape) = enumeration.interface.source_shape()
            else {
                unreachable!("a dependency variant retains its enum source shape")
            };
            let variant = shape
                .variants()
                .iter()
                .find(|value| value.variant() == variant)
                .expect("the shared enum contains its declared variant");
            if variant.style() == hir::EnumSourceVariantStyleV1::Unit {
                self.error_with_note(scoop_source::DiagnosticCode::E0732, span, format!("unit variant `{__diagnostic_arg0}` of `{__diagnostic_arg1}` does not take arguments" , __diagnostic_arg0 = name.text, __diagnostic_arg1 = enumeration.name()), scoop_source::DiagnosticNote::without_location(scoop_source::DiagnosticCode::H0704, format!("use `{__diagnostic_arg2}` without parentheses" , __diagnostic_arg2 = name.text)));
                return None;
            }
            argument_mode = match variant.style() {
                hir::EnumSourceVariantStyleV1::Named => ArgumentMode::NamedOnly,
                hir::EnumSourceVariantStyleV1::Positional => ArgumentMode::PositionalOnly,
                hir::EnumSourceVariantStyleV1::Constructor => ArgumentMode::Mixed,
                hir::EnumSourceVariantStyleV1::Unit => unreachable!("unit variants were rejected"),
            };
        }
        Some(argument_mode)
    }
}
