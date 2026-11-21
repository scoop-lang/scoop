use super::*;
use hir::{EditingHierarchyType, PublicNominalKindV1 as Kind, SourceNominalId as Nominal};
use scoop_identity::SignatureTypeKey;

impl DeclarationAnalysis {
    pub fn hierarchy_type(&self, declaration: Nominal) -> Option<EditingHierarchyType> {
        if !self.declarations_ready
            || !self
                .lowerer
                .editing_type_available(types::TypeTarget::Nominal(declaration))
        {
            return None;
        }
        let kind = if let Some(owner) = self.lowerer.nominal_owners.get(&declaration) {
            match owner {
                Owner::Class(_) => Kind::Class,
                Owner::Interface(_) => Kind::Interface,
                Owner::Struct(_) => Kind::Struct,
                Owner::Enum(_) => Kind::Enum,
                Owner::Object(_) => Kind::Object,
            }
        } else {
            self.lowerer
                .dependencies
                .as_ref()?
                .nominal_declaration(declaration)?
                .interface
                .kind()
        };
        Some(EditingHierarchyType {
            declaration,
            kind,
            name: self.lowerer.nominal_template_name(declaration).to_owned(),
            origin: self.lowerer.semantic_nominal_origin(declaration).ok()?,
        })
    }

    pub fn hierarchy_declaration(
        &self,
        origin: &hir::SemanticSourceLocation,
        is_cancelled: &dyn Fn() -> bool,
    ) -> Result<Option<Nominal>, EditingCancelled> {
        for reference in &self.type_references {
            if is_cancelled() {
                return Err(EditingCancelled);
            }
            if let Identity::Nominal(declaration) = reference.target
                && reference.is_declaration
                && reference.origin.source == origin.source
                && origin.span.start <= reference.origin.span.start
                && reference.origin.span.end <= origin.span.end
                && self
                    .hierarchy_type(declaration)
                    .is_some_and(|node| node.origin == *origin)
            {
                return Ok(Some(declaration));
            }
        }
        if let Some(dependencies) = &self.lowerer.dependencies {
            for declaration in dependencies.nominal_declarations() {
                if is_cancelled() {
                    return Err(EditingCancelled);
                }
                if crate::semantic_queries::imported_location(declaration.origin.origin())
                    .is_ok_and(|location| location == *origin)
                {
                    return Ok(Some(declaration.owner()));
                }
            }
        }
        Ok(None)
    }

    pub fn hierarchy_supertypes(&self, declaration: Nominal) -> Vec<Nominal> {
        if !self.declarations_ready
            || !self
                .lowerer
                .editing_type_available(types::TypeTarget::Nominal(declaration))
        {
            return Vec::new();
        }
        let mut parents = if let Some(&owner) = self.lowerer.nominal_owners.get(&declaration) {
            self.lowerer.declared_hierarchy_parents(owner)
        } else if let Some(imported) = self
            .lowerer
            .dependencies
            .as_ref()
            .and_then(|dependencies| dependencies.nominal_declaration(declaration))
        {
            let record = &imported.interface;
            record
                .exact_supertypes()
                .values()
                .iter()
                .chain(
                    record
                        .declaration_details()
                        .conditional_interfaces()
                        .iter()
                        .map(|conditional| conditional.interface()),
                )
                .filter_map(|ty| match ty {
                    SignatureTypeKey::Nominal(id) => Some(Nominal::Concrete(*id)),
                    SignatureTypeKey::NominalApplication { origin, .. } => {
                        Some(Nominal::GenericTemplate(*origin))
                    }
                    _ => None,
                })
                .collect()
        } else {
            Vec::new()
        };
        parents.retain(|parent| *parent != declaration);
        parents.sort_unstable();
        parents.dedup();
        parents
    }

    pub fn hierarchy_subtypes(
        &mut self,
        declaration: Nominal,
        is_cancelled: &dyn Fn() -> bool,
    ) -> Result<&[Nominal], EditingCancelled> {
        if self.direct_type_children.is_none() {
            let mut children = HashMap::<_, Vec<_>>::new();
            for reference in &self.type_references {
                if is_cancelled() {
                    return Err(EditingCancelled);
                }
                let Identity::Nominal(child) = reference.target else {
                    continue;
                };
                if reference.is_declaration {
                    for parent in self.hierarchy_supertypes(child) {
                        children.entry(parent).or_default().push(child);
                    }
                }
            }
            self.direct_type_children = Some(children);
        }
        Ok(self
            .direct_type_children
            .as_ref()
            .and_then(|children| children.get(&declaration))
            .map_or(&[], Vec::as_slice))
    }
}

impl Lowerer {
    fn declared_hierarchy_parents(&self, owner: Owner) -> Vec<Nominal> {
        let (base, interfaces, conditional) = match owner {
            Owner::Class(id) => {
                let definition = &self.classes[id];
                (
                    definition.base_class,
                    definition.interfaces.as_slice(),
                    definition.conditional_interfaces.as_slice(),
                )
            }
            Owner::Object(id) => {
                return self
                    .declared_hierarchy_parents(Owner::Class(self.objects[id].backing_class));
            }
            Owner::Struct(id) => {
                let definition = &self.structs[id];
                (
                    None,
                    definition.interfaces.as_slice(),
                    definition.conditional_interfaces.as_slice(),
                )
            }
            Owner::Enum(id) => {
                let definition = &self.enums[id];
                (
                    None,
                    definition.interfaces.as_slice(),
                    definition.conditional_interfaces.as_slice(),
                )
            }
            Owner::Interface(id) => (None, self.interfaces[id].parents.as_slice(), &[][..]),
        };
        base.into_iter()
            .chain(interfaces.iter().copied())
            .chain(conditional.iter().map(|declaration| declaration.interface))
            .filter_map(|ty| self.semantic_nominal_identity(ty))
            .collect()
    }
}
