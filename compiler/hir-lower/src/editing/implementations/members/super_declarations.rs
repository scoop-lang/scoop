use super::*;
use crate::defaults::DefaultOverrideSource;
use hir::{PropertyReference, SemanticCallable, SemanticProperty, SemanticSourceLocation};

impl DeclarationAnalysis {
    pub fn super_declarations(
        &mut self,
        source: &scoop_identity::SourceIdentity,
        span: ast::Span,
        is_cancelled: impl Fn() -> bool,
    ) -> Result<Vec<SemanticSourceLocation>, EditingCancelled> {
        if is_cancelled() {
            return Err(EditingCancelled);
        }
        if !self.declarations_ready {
            return Ok(Vec::new());
        }
        if let Some(function) = self.functions.iter().find(|function| {
            &function.name_origin.source == source && function.name_origin.span == span
        }) {
            return self
                .lowerer
                .method_super_declarations(function.function, &is_cancelled);
        }
        if let Some(property) = self.property_declaration_at(source, span) {
            return self
                .lowerer
                .property_super_declarations(property, &is_cancelled);
        }
        Ok(Vec::new())
    }
}

impl Lowerer {
    fn method_super_declarations(
        &mut self,
        function: hir::FunctionId,
        is_cancelled: &impl Fn() -> bool,
    ) -> Result<Vec<SemanticSourceLocation>, EditingCancelled> {
        if self.invalid_override_methods.contains(&function)
            || self
                .declaration_surface
                .unavailable_signature(function)
                .is_some()
        {
            return Ok(Vec::new());
        }
        let Some(sources) = self.override_default_sources.get(&function).cloned() else {
            return Ok(Vec::new());
        };
        let mut parents = Vec::new();
        for source in sources {
            if is_cancelled() {
                return Err(EditingCancelled);
            }
            let (owner, origin) = match source {
                DefaultOverrideSource::Local {
                    function,
                    mut type_arguments,
                } => {
                    let owner = self.function_owner[&function];
                    type_arguments.truncate(self.signatures[&function].owner_type_param_count);
                    let application = self.method_owner_application(owner, type_arguments);
                    (
                        self.method_owner_type(application),
                        self.editing_callable_origin(SemanticCallable::Current(function)),
                    )
                }
                DefaultOverrideSource::Imported { declaration, owner } => {
                    let origin = self
                        .dependencies
                        .as_ref()
                        .and_then(|catalog| catalog.callable_declaration(declaration).ok())
                        .and_then(|declaration| {
                            crate::semantic_queries::imported_location(
                                declaration.definition_origin().origin(),
                            )
                            .ok()
                        });
                    (owner, origin)
                }
            };
            parents.push((owner, origin));
        }
        let mut results = Vec::new();
        for (owner, origin) in &parents {
            if is_cancelled() {
                return Err(EditingCancelled);
            }
            if let Some(origin) = origin
                && !parents.iter().any(|(candidate, _)| {
                    !self.types_equal(*owner, *candidate) && self.is_subtype(*candidate, *owner)
                })
            {
                results.push(origin.clone());
            }
        }
        Ok(results)
    }

    fn property_super_declarations(
        &self,
        property: hir::PropertyId,
        is_cancelled: &impl Fn() -> bool,
    ) -> Result<Vec<SemanticSourceLocation>, EditingCancelled> {
        let declaration = &self.properties[property];
        if !declaration.is_override {
            return Ok(Vec::new());
        }
        let mut local_ancestors = HashSet::new();
        let mut imported_ancestors = HashSet::new();
        for parent in &declaration.overrides {
            if is_cancelled() {
                return Err(EditingCancelled);
            }
            match *parent {
                PropertyReference::Local(parent) => {
                    for ancestor in &self.properties[parent].overrides {
                        match *ancestor {
                            PropertyReference::Local(property) => {
                                local_ancestors.insert(property);
                            }
                            PropertyReference::Imported { declaration, .. } => {
                                imported_ancestors.insert(declaration);
                            }
                        }
                    }
                }
                PropertyReference::Imported { declaration, .. } => {
                    if let Some(parent) = self.dependencies.as_ref().and_then(|catalog| {
                        catalog
                            .property_declaration(hir::PropertyDeclarationId::Property(declaration))
                    }) {
                        imported_ancestors.extend(parent.overrides().iter().copied());
                    }
                }
            }
        }
        Ok(declaration
            .overrides
            .iter()
            .filter_map(|parent| {
                let property = match *parent {
                    PropertyReference::Local(property) if !local_ancestors.contains(&property) => {
                        SemanticProperty::Current(property)
                    }
                    PropertyReference::Imported { declaration, .. }
                        if !imported_ancestors.contains(&declaration) =>
                    {
                        SemanticProperty::Imported(hir::PropertyDeclarationId::Property(
                            declaration,
                        ))
                    }
                    _ => return None,
                };
                self.editing_property_origin(property)
            })
            .collect())
    }
}
