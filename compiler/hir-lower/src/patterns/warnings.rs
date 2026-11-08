//! Binding warnings use the same visible names and exact enum as ordinary lookup.

use super::*;
use crate::imports::lookup::{
    LookupResult,
    values::{ValueOrigin, ValueTarget},
};

impl Lowerer {
    pub(crate) fn warn_variant_shadowing(&mut self, name: &ast::Ident, ty: TypeId, in_match: bool) {
        let match_variant = in_match && self.named_enum_variant(ty, &name.text).is_some();
        if match_variant
            || (self.binding_name_has_variant(&name.text, ty, None)
                && !self.lexical_or_member_value_blocks_type_qualifier(&name.text))
        {
            self.variant_shadowing_warning(name);
        }
    }

    pub(crate) fn warn_top_level_variant_shadowing(
        &mut self,
        declaration: &ast::PropertyDecl,
        file: usize,
    ) {
        let Some(property) = self.property_declaration_id(declaration, file) else {
            return;
        };
        if matches!(
            self.properties[property].owner,
            hir::PropertyOwner::TopLevel
        ) && self.binding_name_has_variant(
            &declaration.name.text,
            self.properties[property].ty,
            Some(property),
        ) {
            self.variant_shadowing_warning(&declaration.name);
        }
    }

    fn binding_name_has_variant(
        &self,
        name: &str,
        ty: TypeId,
        excluded_property: Option<hir::PropertyId>,
    ) -> bool {
        match self
            .lookup_value_origin_excluding_property(name, excluded_property)
            .1
        {
            LookupResult::Unique(ValueOrigin::Dependency(binding)) => {
                matches!(binding.target(), hir::ImportedTarget::EnumVariant(_))
            }
            LookupResult::Unique(origin) => matches!(
                self.materialized_value_target(&origin),
                Some(ValueTarget::Variant(_))
            ),
            LookupResult::Missing => self.named_enum_variant(ty, name).is_some(),
            LookupResult::Unavailable(_)
            | LookupResult::Ambiguous { .. }
            | LookupResult::Inaccessible(_) => false,
        }
    }

    fn variant_shadowing_warning(&mut self, name: &ast::Ident) {
        self.warnings.push(ast::Diagnostic::warning_at_file(
            self.current_file,
            name.span,
            format!(
                "binding `{}` shadows an enum variant available here; rename the binding or qualify the variant",
                name.text,
            ),
        ));
    }
}
