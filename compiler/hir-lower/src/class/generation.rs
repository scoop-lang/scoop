//! Ordinary missing interface methods retained for the driver generation step.

use super::inheritance::InterfaceMemberInstance;
use super::*;
use crate::generation::PendingGenerationMethod;
use hir::ImportedCallableSource;

impl Lowerer {
    pub(in crate::class) fn declare_generated_interface_method(
        &mut self,
        owner: Owner,
        member: &InterfaceMemberInstance,
        candidates: &[crate::CallableCandidate],
        span: ast::Span,
        host: &str,
    ) -> Option<FunctionId> {
        if !self.generation.enabled
            || member.signature.accessor.is_some()
            || member.signature.result_pending
            || member.signature.parameter_pending
            || self.coding_candidate_declared(
                &member.signature.name,
                &member.signature.parameters,
                candidates,
            )
        {
            return None;
        }
        let signature = self.interface_forwarder_signature(member, span);
        let function = self.allocate_synthesized_method(
            owner,
            &member.signature.name,
            signature,
            hir::MethodModifier::Final,
            ast::DeclaredVisibility::Public,
            span,
            crate::FunctionOrigin::Source(crate::SourceFunctionDeclaration {
                name: member.signature.name.clone(),
                name_span: None,
            }),
        );

        let mut diagnostic = ast::Diagnostic::at(
            scoop_source::DiagnosticCode::E1008,
            span,
            format!(
                "{host} does not implement interface method `{}.{}`",
                self.type_name(member.owner),
                member.signature.name,
            ),
        );
        diagnostic.set_file(self.current_file);
        self.generation.methods.insert(
            function,
            PendingGenerationMethod {
                task: hir::PendingMethod {
                    function,
                    owner: owner.as_nominal_owner(),
                    requirements: Vec::new(),
                },
                diagnostic,
            },
        );
        self.record_generated_requirement(function, owner, member);
        Some(function)
    }

    pub(in crate::class) fn record_generated_requirement(
        &mut self,
        function: FunctionId,
        owner: Owner,
        member: &InterfaceMemberInstance,
    ) {
        let Some(pending) = self.generation.methods.get_mut(&function) else {
            return;
        };
        if pending.task.owner != owner.as_nominal_owner() {
            return;
        }
        let requirement = hir::AppliedRequirement {
            interface: member.owner,
            declaration: member.member,
        };
        if pending.task.requirements.contains(&requirement) {
            return;
        }
        pending.task.requirements.push(requirement);
        let source = match member.member {
            hir::InterfaceMethodReference::Local(method) => {
                let Type::Interface(application) = self.types[member.owner] else {
                    unreachable!("a requirement retains its interface application")
                };
                crate::defaults::DefaultOverrideSource::Local {
                    function: self.interface_method_entities[method].function,
                    type_arguments: self.interface_applications[application].arguments.clone(),
                }
            }
            hir::InterfaceMethodReference::Imported { .. } => {
                let declaration = self.delegated_imported_member(member);
                crate::defaults::DefaultOverrideSource::Imported {
                    declaration: declaration.interface().declaration(),
                    owner: member.owner,
                }
            }
        };
        self.override_default_sources
            .entry(function)
            .or_default()
            .push(source);
    }

    pub(crate) fn resolve_implemented_interface(
        &mut self,
        owner: hir::NominalOwner,
        syntax: &ast::TypeRef,
    ) -> Result<hir::InterfaceSelection, Vec<ast::Diagnostic>> {
        let context = hir::SemanticContext::Nominal(owner);
        let owner = Owner::from_nominal_owner(owner);
        let resolve = |state: &mut Self| {
            state.type_query(context.into(), syntax.span, |state| {
                state.resolve_type_ref(syntax)
            })
        };
        let errors = match resolve(self) {
            Ok(ty) => {
                if !matches!(self.types[ty], Type::Interface(_)) {
                    return Ok(hir::InterfaceSelection::NotInterface);
                }
                let selected = self
                    .owner_conformances(owner)
                    .iter()
                    .find(|implementation| self.types_equal(ty, implementation.interface));
                return Ok(selected
                    .map_or(hir::InterfaceSelection::NotImplemented, |implementation| {
                        hir::InterfaceSelection::Implemented(implementation.interface)
                    }));
            }
            Err(errors) => errors,
        };

        // A projection or bounded type argument may only be well-formed in
        // the implementation that declares it. Keep each domain independent.
        let conditional: Vec<_> = self
            .owner_conformances(owner)
            .iter()
            .filter_map(|implementation| {
                Some((implementation.interface, implementation.condition.clone()?))
            })
            .collect();
        for (interface, condition) in conditional {
            let result = self.with_conditional_interface_assumptions(&condition, resolve);
            if result.is_ok_and(|ty| self.types_equal(ty, interface)) {
                return Ok(hir::InterfaceSelection::Implemented(interface));
            }
        }
        Err(errors)
    }

    pub(crate) fn query_interface_requirements(
        &mut self,
        interface: TypeId,
    ) -> Vec<hir::AppliedRequirement> {
        self.conformance_members(interface)
            .into_iter()
            .map(|member| hir::AppliedRequirement {
                interface: member.owner,
                declaration: member.member,
            })
            .collect()
    }
}
