use hir::{
    DefaultCallableDeclarationV1 as Declaration, EditingCallee as C,
    EditingPropertyAccess as Access, ImportedCallableSource, SemanticConstructor as Constructor,
    SemanticProperty as Property,
};
use scoop_identity::CallableTemplateOrigin;

use super::*;
use crate::Owner;

impl DeclarationAnalysis {
    pub(super) fn call_hierarchy_node(&self, callee: C) -> Option<Node> {
        let callee = match callee {
            C::Constructor(Constructor::CurrentClass(constructor)) => {
                match self.lowerer.class_constructors[constructor].identity_kind {
                    hir::ClassConstructorIdentityKind::Source => callee,
                    hir::ClassConstructorIdentityKind::ZeroArgumentAdapter { source } => {
                        C::Constructor(Constructor::CurrentClass(source))
                    }
                }
            }
            _ => callee,
        };
        let (name, kind) = match callee {
            C::Callable(Callable::Current(function)) => {
                if !matches!(self.signature(function), V::Known(_)) {
                    return None;
                }
                (
                    self.lowerer.source_function_name(function).to_owned(),
                    if self.lowerer.functions[function].method.is_some() {
                        Kind::Method
                    } else {
                        Kind::Function
                    },
                )
            }
            C::Callable(Callable::Imported(callable)) => {
                let id = match callable {
                    Declaration::Function(id) => CallableTemplateOrigin::Function(id),
                    Declaration::GenericFunction(id) => CallableTemplateOrigin::GenericFunction(id),
                    _ => return None,
                };
                let declaration = self
                    .lowerer
                    .dependencies
                    .as_ref()?
                    .callable_declaration(id)
                    .ok()?;
                let method = matches!(
                    declaration.interface().owner(),
                    hir::PublicDeclarationOwnerV1::Nominal(_)
                );
                (
                    declaration.name().to_owned(),
                    if method { Kind::Method } else { Kind::Function },
                )
            }
            C::Callable(accessor) => {
                let (property, _) =
                    self.lowerer
                        .properties
                        .iter()
                        .find(|(_, property)| match accessor {
                            Callable::CurrentGetter(id) => property.capability.getter() == id,
                            Callable::CurrentSetter(id) => property.capability.setter() == Some(id),
                            _ => false,
                        })?;
                return self
                    .call_hierarchy_property(Property::Current(property), Access::ReadWrite);
            }
            C::Constructor(Constructor::CurrentClass(constructor)) => (
                self.lowerer.classes[self.lowerer.class_constructors[constructor].owner]
                    .name
                    .clone(),
                Kind::Constructor,
            ),
            C::Constructor(Constructor::CurrentStruct(constructor)) => (
                self.lowerer.structs[self.lowerer.struct_constructors[constructor].owner]
                    .name
                    .clone(),
                Kind::Constructor,
            ),
            C::CurrentVariant(variant) => {
                let variant = &self.lowerer.enums[variant.enumeration()].variants
                    [variant.local_index() as usize];
                (variant.name.clone(), Kind::Constructor)
            }
            C::Constructor(Constructor::Imported(constructor)) => (
                self.imported_constructor_name(CallableTemplateOrigin::Constructor(constructor))?,
                Kind::Constructor,
            ),
            C::ImportedVariant(variant) => (
                self.imported_constructor_name(CallableTemplateOrigin::VariantConstructor(
                    variant,
                ))?,
                Kind::Constructor,
            ),
        };
        Some(Node {
            name,
            kind,
            origin: self.lowerer.editing_callee_origin(callee)?,
        })
    }

    fn imported_constructor_name(&self, origin: CallableTemplateOrigin) -> Option<String> {
        self.lowerer
            .dependencies
            .as_ref()?
            .callable_declaration(origin)
            .ok()
            .map(|declaration| declaration.name().to_owned())
    }

    pub(super) fn call_hierarchy_constructor_at(
        &self,
        source: &SourceIdentity,
        span: ast::Span,
    ) -> Option<Node> {
        if !self.declarations_ready {
            return None;
        }
        let owner = self.type_references.iter().find_map(|reference| {
            if !reference.is_declaration
                || &reference.origin.source != source
                || reference.origin.span != span
            {
                return None;
            }
            let hir::EditingTypeIdentity::Nominal(nominal) = reference.target else {
                return None;
            };
            self.lowerer.nominal_owners.get(&nominal).copied()
        });
        self.lowerer
            .editing_source_constructors()
            .into_iter()
            .find_map(|(callee, _)| {
                let node = self.call_hierarchy_node(callee)?;
                if &node.origin.source != source || !self.declaration_scope_available(&node.origin)
                {
                    return None;
                }
                let at_type_name = match (callee, owner) {
                    (C::Constructor(Constructor::CurrentClass(id)), Some(Owner::Class(owner))) => {
                        let constructor = &self.lowerer.class_constructors[id];
                        constructor.owner == owner
                            && matches!(constructor.kind, hir::ClassConstructorKind::Primary { .. })
                    }
                    (
                        C::Constructor(Constructor::CurrentStruct(id)),
                        Some(Owner::Struct(owner)),
                    ) => {
                        let constructor = &self.lowerer.struct_constructors[id];
                        constructor.owner == owner
                            && matches!(constructor.kind, hir::StructConstructorKind::Primary)
                    }
                    _ => false,
                };
                (at_type_name || node.origin.span == span).then_some(node)
            })
    }

    pub(super) fn call_hierarchy_property(&self, target: Property, access: Access) -> Option<Node> {
        let (name, getter, setter) = match target {
            Property::Current(property) => {
                let property = &self.lowerer.properties[property];
                let getter = &self.lowerer.property_getters[property.capability.getter()];
                (
                    property.name.clone(),
                    accessor_call(getter.implementation),
                    property.capability.setter().is_some_and(|id| {
                        accessor_call(self.lowerer.property_setters[id].implementation)
                    }),
                )
            }
            Property::Imported(property) => {
                let dependencies = self.lowerer.dependencies.as_ref()?;
                let declaration = dependencies.property_declaration(property)?;
                (
                    dependencies.property_name(property)?.to_owned(),
                    declaration
                        .accessors()
                        .getter_source()
                        .implementation()
                        .requires_body(),
                    declaration
                        .accessors()
                        .setter_source()
                        .is_some_and(|setter| setter.implementation().requires_body()),
                )
            }
        };
        let calls = match access {
            Access::Read => getter,
            Access::Write => setter,
            Access::ReadWrite => getter || setter,
        };
        if !calls {
            return None;
        }
        Some(Node {
            name,
            kind: Kind::Property,
            origin: self.lowerer.editing_property_origin(target)?,
        })
    }
}

fn accessor_call(implementation: hir::PropertyAccessorImplementation) -> bool {
    matches!(
        implementation,
        hir::PropertyAccessorImplementation::StorageBody(_)
            | hir::PropertyAccessorImplementation::Body(_)
            | hir::PropertyAccessorImplementation::AbstractSlot(_)
    )
}
