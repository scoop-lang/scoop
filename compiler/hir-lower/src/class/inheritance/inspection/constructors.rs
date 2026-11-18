use super::*;
use crate::call_resolution::candidates::NominalConstructorSource;
use crate::constructor_resolution::NominalConstructorCall;
use crate::expr::CallSite;

enum InspectionConstructor {
    Current(NominalConstructorSource),
    Dependency(Box<hir::ImportedCallableDeclaration>),
}

impl Lowerer {
    fn inspection_constructor(
        &mut self,
        result: TypeId,
        span: Span,
    ) -> Option<InspectionConstructor> {
        let application = self
            .nominal_application(result)
            .expect("a primary constructor has a nominal owner");
        let local = match self.types[result] {
            Type::Struct(_) => self.source_struct_id(application.template).map(|owner| {
                self.struct_primary_constructor(owner)
                    .map(NominalConstructorSource::Struct)
            }),
            Type::Class(_) => self.source_class_id(application.template).map(|owner| {
                self.classes[owner]
                    .constructors
                    .iter()
                    .copied()
                    .find(|id| {
                        let value = &self.class_constructors[*id];
                        value.identity_kind == hir::ClassConstructorIdentityKind::Source
                            && matches!(value.kind, hir::ClassConstructorKind::Primary { .. })
                    })
                    .map(NominalConstructorSource::Class)
            }),
            _ => unreachable!("only structs and classes have primary constructors"),
        };
        if let Some(local) = local {
            let Some(source) = local else {
                self.error(
                    scoop_source::DiagnosticCode::E1310,
                    span,
                    format!(
                        "automatic inspect requires a primary constructor for {}",
                        self.type_name(result)
                    ),
                );
                return None;
            };
            return Some(InspectionConstructor::Current(source));
        }
        let declaration = self
            .dependencies
            .as_ref()?
            .nominal_declaration(application.template)?;
        let details = declaration.interface.declaration_details();
        let constructor = match self.types[result] {
            Type::Struct(_) => details.primary_value_constructor(),
            Type::Class(_) => details
                .class_primary_constructor()
                .map(|primary| primary.constructor()),
            _ => unreachable!("only structs and classes have primary constructors"),
        };
        let Some(constructor) = constructor else {
            self.error(
                scoop_source::DiagnosticCode::E1310,
                span,
                format!(
                    "automatic inspect requires a primary constructor for {}",
                    self.type_name(result)
                ),
            );
            return None;
        };
        let callable = self
            .dependencies
            .as_ref()?
            .callable_declaration(scoop_identity::CallableTemplateOrigin::Constructor(
                constructor,
            ))
            .expect("the primary constructor is declared")
            .clone();
        Some(InspectionConstructor::Dependency(Box::new(callable)))
    }

    pub(in crate::class::inheritance) fn construct_inspection_value(
        &mut self,
        result: TypeId,
        values: Vec<hir::Expr>,
        span: Span,
        sink: &mut Vec<hir::Statement>,
    ) -> Option<hir::Expr> {
        let constructor = self.inspection_constructor(result, span)?;
        self.push_scope();
        let arguments = self.coding_arguments(values, span, sink);
        let value = match constructor {
            InspectionConstructor::Current(source) => {
                let owner_arguments = self
                    .nominal_application(result)
                    .expect("a constructor has a nominal result")
                    .arguments;
                self.resolve_nominal_constructor_overload(
                    &self.type_name(result),
                    &[source],
                    NominalConstructorCall {
                        enclosing: None,
                        explicit_type_args: &[],
                        expected_type_args: Some(&owner_arguments),
                        arguments: &arguments,
                        span,
                    },
                    sink,
                )
                .map(|resolved| {
                    let kind = match resolved.source {
                        NominalConstructorSource::Struct(constructor) => {
                            let Type::Struct(application) = self.types[result] else {
                                unreachable!("a struct constructor returns its owner")
                            };
                            hir::ExprKind::StructInit {
                                constructor: self
                                    .struct_constructor_application(constructor, application),
                                args: resolved.args,
                            }
                        }
                        NominalConstructorSource::Class(constructor) => {
                            let Type::Class(application) = self.types[result] else {
                                unreachable!("a class constructor returns its owner")
                            };
                            hir::ExprKind::ClassInit {
                                constructor: self
                                    .class_constructor_application(constructor, application),
                                args: resolved.args,
                            }
                        }
                        _ => unreachable!("inspection constructs a core struct or class"),
                    };
                    self.coding_expr(kind, result, span)
                })
            }
            InspectionConstructor::Dependency(declaration) => {
                match self.probe_imported_value_constructor(
                    declaration.as_ref().clone(),
                    &ast::Ident {
                        text: declaration.name().into(),
                        span,
                    },
                    CallSite {
                        type_args: &[],
                        args: &arguments,
                        span,
                    },
                    Some(result),
                ) {
                    Ok(probe) => self.commit_imported_dependency_callable(probe, sink),
                    Err(failure) => {
                        self.commit_layer_diagnostics(&failure);
                        None
                    }
                }
            }
        };
        self.pop_scope();
        value
    }
}
