//! Import suggestions use the same stored namespaces as ordinary source imports.

use super::*;
use crate::Lowerer;
use crate::imports::lookup::values::{ValueOrigin, ValueTarget};
use hir::ImportedCallableSource;
use std::collections::BTreeSet;

#[derive(Clone, Copy)]
pub(crate) enum MissingNameUse {
    Type,
    Value,
    Call,
    Method(hir::TypeId),
    Property(hir::TypeId),
}

impl Lowerer {
    pub(crate) fn missing_name_error(
        &mut self,
        name: &ast::Ident,
        message: String,
        usage: MissingNameUse,
    ) {
        let notes = self.import_suggestions(name, usage);
        let mut diagnostic = ast::Diagnostic::at_file(self.current_text_file(), name.span, message);
        diagnostic.notes = notes;
        self.diagnostics.push(diagnostic);
    }

    pub(crate) fn append_import_suggestions(&mut self, name: &ast::Ident, usage: MissingNameUse) {
        let notes = self.import_suggestions(name, usage);
        if let Some(diagnostic) = self.diagnostics.last_mut() {
            diagnostic.notes.extend(notes);
        }
    }

    fn import_suggestions(
        &self,
        name: &ast::Ident,
        usage: MissingNameUse,
    ) -> Vec<ast::DiagnosticNote> {
        // Generated bodies have a different lexical header. An import edit
        // there cannot repair the original file's lookup context.
        if self.generated_source_file.is_some() || self.import_suggestion_blocked(&name.text) {
            return Vec::new();
        }
        let paths = self.imports.suggestion_paths(self, &name.text);
        let mut probe = self.probe_state();
        let mut notes = Vec::new();
        for path in paths {
            let syntax = selector_path(&path, name.span);
            let selected = self.imports.selector(self, None, &syntax, false);
            let Ok(selector::SelectorResult::Targets(targets)) = selected else {
                continue;
            };
            let Some(binding) = targets
                .iter()
                .find(|binding| probe.import_matches(binding, usage))
            else {
                continue;
            };
            let mut note = match binding {
                ImportedTargetBinding::CurrentCone { binding, .. } => {
                    let binding = self.imports.binding(*binding);
                    let mut note = ast::DiagnosticNote::at(binding.file, binding.span, "");
                    note.source = Some(Box::new(binding.source.clone()));
                    note
                }
                ImportedTargetBinding::DirectDependency(binding) => {
                    let Ok(note) = self.dependency_candidate_note(binding.target()) else {
                        continue;
                    };
                    note
                }
            };
            let rendered = path
                .iter()
                .map(|part| format!("`{part}`"))
                .collect::<Vec<_>>()
                .join(".");
            note.message = format!("available with import {rendered}");
            note.import_path = path;
            notes.push(note);
        }
        notes
    }

    fn import_suggestion_blocked(&self, name: &str) -> bool {
        let Some(file) = self.imports.files.get(&self.current_file) else {
            return true;
        };
        file.exact.iter().any(|import| import.local_name == name)
            || file
                .unavailable
                .iter()
                .any(|import| import.local_name.as_deref() == Some(name))
            || self
                .imports
                .block_layers(self, Some(name))
                .iter()
                .any(|layer| {
                    !layer.bindings.is_empty()
                        || !layer.dependency_bindings.is_empty()
                        || !layer.suppressed_values.is_empty()
                        || !layer.suppressed_callables.is_empty()
                })
    }

    fn import_matches(&mut self, binding: &ImportedTargetBinding, usage: MissingNameUse) -> bool {
        match binding {
            ImportedTargetBinding::CurrentCone { binding, .. } => {
                self.current_import_matches(*binding, usage)
            }
            ImportedTargetBinding::DirectDependency(binding) => {
                self.dependency_import_matches(binding, usage)
            }
        }
    }

    fn current_import_matches(
        &mut self,
        binding: CurrentUnitBindingId,
        usage: MissingNameUse,
    ) -> bool {
        use CurrentUnitTarget as T;
        use MissingNameUse as U;
        let target = self.imports.binding(binding).target;
        match (target, usage) {
            (T::Unavailable(_) | T::UnavailableImport, _) => false,
            (target, U::Type) => target.occupies_namespace(scoop_identity::BindingNamespace::Type),
            (T::Function(function), U::Call) => {
                !self.extension_receivers.contains_key(&function)
                    && !self.declaration_surface.rejects_function(function)
            }
            (
                T::Class(_)
                | T::Struct(_)
                | T::TypeAlias(_)
                | T::EnumVariant(_)
                | T::SourceVariant(_),
                U::Call,
            ) => true,
            (T::Object(_) | T::EnumVariant(_) | T::SourceVariant(_), U::Value) => true,
            (T::Property(_) | T::SourceProperty(_), U::Value | U::Property(_)) => {
                let Some(ValueTarget::Property(property)) =
                    self.materialized_value_target(&ValueOrigin::CurrentUnit(binding))
                else {
                    return false;
                };
                let extension = matches!(
                    self.properties[property].owner,
                    hir::PropertyOwner::Extension(_)
                );
                match usage {
                    U::Value => !extension,
                    U::Property(receiver) if extension => self
                        .source_extension_receiver_applicable(
                            self.extension_property_getter(property),
                            receiver,
                        )
                        .unwrap_or(false),
                    _ => false,
                }
            }
            (T::Function(function), U::Method(receiver))
                if self.extension_receivers.contains_key(&function) =>
            {
                self.source_extension_receiver_applicable(function, receiver)
                    .unwrap_or(false)
            }
            _ => false,
        }
    }

    fn dependency_import_matches(
        &mut self,
        binding: &hir::DirectImportedTargetBinding,
        usage: MissingNameUse,
    ) -> bool {
        use MissingNameUse as U;
        use hir::ImportedTarget as T;
        let target = binding.target();
        match usage {
            U::Type => matches!(
                target,
                T::Type(_) | T::GenericType(_) | T::TypeAlias(_) | T::Annotation(_)
            ),
            U::Value => matches!(
                target,
                T::Property(_) | T::ObjectValue(_) | T::EnumVariant(_)
            ),
            U::Call
                if matches!(
                    target,
                    T::Type(_) | T::GenericType(_) | T::TypeAlias(_) | T::EnumVariant(_)
                ) =>
            {
                true
            }
            U::Call | U::Method(_) if matches!(target, T::Function(_) | T::GenericFunction(_)) => {
                let Some(declaration) = self.dependencies.as_ref().and_then(|dependencies| {
                    let candidate = dependencies.callable_candidate(binding).ok()?;
                    dependencies
                        .callable_declaration(candidate.interface().declaration())
                        .ok()
                }) else {
                    return false;
                };
                match usage {
                    U::Call => declaration.interface().receiver().is_none(),
                    U::Method(receiver) if declaration.interface().receiver().is_some() => self
                        .imported_extension_receiver_applicable(&declaration, receiver)
                        .unwrap_or(false),
                    _ => false,
                }
            }
            U::Property(receiver) if matches!(target, T::ExtensionProperty(_)) => {
                let Some(getter) = self.dependencies.as_ref().and_then(|dependencies| {
                    let property = dependencies.property_candidate(binding).ok()?;
                    dependencies
                        .callable_declaration(scoop_identity::CallableTemplateOrigin::Accessor(
                            property.interface().accessors().getter(),
                        ))
                        .ok()
                }) else {
                    return false;
                };
                self.imported_extension_receiver_applicable(&getter, receiver)
                    .unwrap_or(false)
            }
            _ => false,
        }
    }
}

impl CurrentUnitImports {
    fn suggestion_paths(&self, lowerer: &Lowerer, name: &str) -> BTreeSet<Vec<String>> {
        let mut paths = BTreeSet::new();
        for (namespace, members) in &self.namespaces {
            let ResolvedNamespace::Package(package) = namespace else {
                continue;
            };
            if members.contains_key(name) {
                let mut path = lowerer
                    .top_level_namespaces
                    .package_segments(*package)
                    .iter()
                    .map(|part| (*part).to_owned())
                    .collect::<Vec<_>>();
                path.push(name.to_owned());
                paths.insert(path);
            }
        }
        for (package, contributions) in self.direct_packages.iter() {
            if contributions.iter().any(|contribution| {
                lowerer.dependency_in_source_scope(contribution.provider)
                    && contribution
                        .groups
                        .iter()
                        .any(|group| group.name().as_str() == name)
            }) {
                let mut path = package
                    .segments()
                    .iter()
                    .map(|part| part.as_str().to_owned())
                    .collect::<Vec<_>>();
                path.push(name.to_owned());
                paths.insert(path);
            }
        }
        paths
    }
}

fn selector_path(path: &[String], span: ast::Span) -> ast::QualifiedNameSyntax {
    let ident = |text: &String| ast::Ident {
        text: text.clone(),
        span,
    };
    ast::QualifiedNameSyntax {
        first: ident(&path[0]),
        rest: path[1..]
            .iter()
            .map(|part| ast::QualifiedNameTailSyntax {
                dot_span: span,
                identifier: ident(part),
            })
            .collect(),
        span,
    }
}
