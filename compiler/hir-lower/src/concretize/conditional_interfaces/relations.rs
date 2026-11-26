//! Proofs read checked declarations, including while a concrete table is being built.

use super::*;

struct NominalRelations<'a> {
    ordinary: Vec<export::TypeId>,
    conditional: &'a [export::ConditionalInterfaceDeclaration],
    arguments: Vec<concrete::TypeId>,
}

impl Concretizer<'_> {
    pub(super) fn concrete_subtype(
        &mut self,
        source: concrete::TypeId,
        target: concrete::TypeId,
    ) -> bool {
        if source == target
            || self.is_nothing_type(source)
            || matches!(self.types[target].kind, concrete::TypeKind::Any)
        {
            return true;
        }
        if !self.active_conditional_relations.insert((source, target)) {
            return false;
        }
        let result = self.concrete_subtype_from_declaration(source, target);
        self.active_conditional_relations.remove(&(source, target));
        result
    }

    fn concrete_subtype_from_declaration(
        &mut self,
        source: concrete::TypeId,
        target: concrete::TypeId,
    ) -> bool {
        let Some(relations) = self.concrete_nominal_relations(source) else {
            return false;
        };
        for parent in relations.ordinary {
            let parent = self.lower_type(parent, &relations.arguments);
            if self.concrete_subtype(parent, target) {
                return true;
            }
        }
        if matches!(self.types[target].kind, concrete::TypeKind::Interface(_)) {
            for declaration in relations.conditional {
                // Its target can contain a companion projection whose existence is conditional.
                if self.concrete_interface_condition_holds(declaration, &relations.arguments) {
                    let interface = self.lower_type(declaration.interface, &relations.arguments);
                    if self.concrete_subtype(interface, target) {
                        return true;
                    }
                }
            }
        }
        false
    }
}

impl<'a> Concretizer<'a> {
    fn concrete_nominal_relations(&self, ty: concrete::TypeId) -> Option<NominalRelations<'a>> {
        let (ordinary, conditional): (_, &[_]) = match self.types[ty].kind {
            concrete::TypeKind::Class(id) => {
                let concrete = &self.classes[id];
                let source = self
                    .source
                    .class_definition(concrete.origin.declaration_id());
                (
                    source
                        .base_class
                        .into_iter()
                        .chain(source.interfaces.iter().copied())
                        .collect(),
                    &source.conditional_interfaces,
                )
            }
            concrete::TypeKind::Struct(_) | concrete::TypeKind::Ptr(_) => {
                let owner = self.type_nominal(ty).expect("a struct has a source owner");
                let source = self.source.struct_definition(owner);
                (source.interfaces.clone(), &source.conditional_interfaces)
            }
            concrete::TypeKind::Enum(id) => {
                let concrete = &self.enums[id];
                let source = self
                    .source
                    .enum_definition(concrete.origin.declaration_id());
                let mut ordinary = source.interfaces.clone();
                if let Some(kind) = source.representation.integer_kind() {
                    ordinary.extend_from_slice(self.source.integer_interface_roots(kind));
                }
                (ordinary, &source.conditional_interfaces)
            }
            concrete::TypeKind::Interface(id) => {
                let source = self
                    .source
                    .interface_definition(self.interfaces[id].origin.declaration_id());
                (source.parents.clone(), &[])
            }
            concrete::TypeKind::Unit
            | concrete::TypeKind::Integer(_)
            | concrete::TypeKind::Boolean
            | concrete::TypeKind::String => {
                let (interfaces, _) = self.scalar_conformance_source(ty);
                (interfaces.to_vec(), &[])
            }
            concrete::TypeKind::Any
            | concrete::TypeKind::Tuple(_)
            | concrete::TypeKind::Function(_)
            | concrete::TypeKind::FunPtr(_)
            | concrete::TypeKind::GcLeafFunPtr(_) => return None,
        };
        Some(NominalRelations {
            ordinary,
            conditional,
            arguments: self.type_arguments(ty),
        })
    }
}
