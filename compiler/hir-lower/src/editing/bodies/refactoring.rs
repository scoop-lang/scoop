use super::*;
use hir::ImportedCallableSource;
use scoop_identity::{CallableTemplateOrigin, SignatureTypeKey};

impl DeclarationAnalysis {
    /// Compare checked types across source revisions without comparing arena ids.
    pub fn expression_type_for_edit(
        &mut self,
        body: hir::EditingSourceBodyId,
        span: ast::Span,
    ) -> Option<(SignatureTypeKey, Option<String>)> {
        let V::Known(observed) = self.body_facts.get(&body)? else {
            return None;
        };
        let V::Known(facts) = &observed.facts else {
            return None;
        };
        let expression = facts
            .expressions
            .iter()
            .find(|expression| expression.span == span)?;
        let V::Known(expected) = expression.expected_type else {
            return None;
        };
        let context = expression.context;
        let parameters = &observed.contexts.get(&context)?.parameters;
        let binders = parameters
            .iter()
            .enumerate()
            .map(|(index, parameter)| hir::HirSignatureBinder {
                parameter: parameter.id,
                depth: 0,
                index: index as u32,
            })
            .collect::<Vec<_>>();
        let mapper = crate::persistent_types::SignatureTypeMapper::new(
            &self.lowerer,
            self.lowerer.nominal_identities.as_ref()?,
            self.lowerer.query_core_types(),
        );
        let key = mapper.map(expression.value_type, &binders).ok()?;
        let syntax = match expected {
            Some(expected) => {
                let ty = if mapper.map(expected, &binders).ok()? == key {
                    expected
                } else {
                    expression.value_type
                };
                match self.describe_body_type(body, context, ty).ok()? {
                    V::Known(description) => Some(description.name),
                    V::Unavailable(_) => return None,
                }
            }
            None => None,
        };
        Some((key, syntax))
    }

    pub fn valid_extracted_call(
        &self,
        body: hir::EditingSourceBodyId,
        application: ast::Span,
        declaration_header: ast::Span,
        require_fallthrough: bool,
    ) -> bool {
        let Some(V::Known(observed)) = self.body_facts.get(&body) else {
            return false;
        };
        let V::Known(facts) = &observed.facts else {
            return false;
        };
        let Some(hir::SemanticCallable::Current(function)) = observed
            .calls
            .iter()
            .find(|call| call.span == application)
            .and_then(|call| call.callee?.callable())
        else {
            return false;
        };
        if !facts.local_functions.iter().any(|local| {
            local.function == function
                && declaration_header.start <= local.name_origin.span.start
                && local.name_origin.span.end <= declaration_header.end
        }) {
            return false;
        }
        !require_fallthrough
            || matches!(
                &self.lowerer.functions[function].kind,
                hir::FunctionKind::User(body)
                    if self.lowerer.statements_control_outcomes(&body.statements).can_fall_through()
            )
    }

    pub fn is_source_location_call(&self, target: hir::EditingCallee) -> bool {
        use hir::{DefaultCallableDeclarationV1 as D, SemanticCallable};
        let Some(callable) = target.callable() else {
            return false;
        };
        let origin = match callable {
            SemanticCallable::Current(id) => {
                return matches!(&self.lowerer.functions[id].kind, hir::FunctionKind::Intrinsic(intrinsic)
                    if intrinsic.kind == hir::IntrinsicFunctionKind::CurrentSourceLocation);
            }
            SemanticCallable::Imported(D::Function(id)) => CallableTemplateOrigin::Function(id),
            SemanticCallable::Imported(D::GenericFunction(id)) => {
                CallableTemplateOrigin::GenericFunction(id)
            }
            _ => return false,
        };
        self.lowerer
            .dependencies
            .as_ref()
            .and_then(|catalog| catalog.callable_declaration(origin).ok())
            .is_some_and(|declaration| {
                matches!(
                    declaration.interface().effects().implementation(),
                    hir::CallableImplementationV1::Intrinsic(
                        hir::IntrinsicFunctionKind::CurrentSourceLocation
                    )
                )
            })
    }

    pub fn has_source_location_default(
        &self,
        body: hir::EditingSourceBodyId,
        moved: ast::Span,
    ) -> bool {
        let Some(V::Known(observed)) = self.body_facts.get(&body) else {
            return false;
        };
        observed
            .source_location_defaults
            .iter()
            .any(|call| moved.start <= call.start && call.end <= moved.end)
    }
}

impl Lowerer {
    pub(crate) fn observe_default_source_location(
        &mut self,
        expression: &hir::ExprKind,
        call: ast::Span,
    ) {
        use crate::imported_generics::PreparedImportedCallableSource;
        use hir::IntrinsicFunctionKind::CurrentSourceLocation;

        if !self.observes_current_body() {
            return;
        }
        let hir::ExprKind::Call { callee, .. } = expression else {
            return;
        };
        let location = match callee {
            hir::CallableTarget::Local(callee) => {
                let function = self.callable_function_id(*callee);
                matches!(&self.functions[function].kind, hir::FunctionKind::Intrinsic(intrinsic)
                    if intrinsic.kind == CurrentSourceLocation)
            }
            hir::CallableTarget::Application(application) => {
                let template = self.imported_generic_applications[*application].template;
                matches!(&self.imported_generic_templates[template].source,
                    PreparedImportedCallableSource::Declaration(declaration)
                        if declaration.interface().effects().implementation()
                            == hir::CallableImplementationV1::Intrinsic(CurrentSourceLocation))
            }
            hir::CallableTarget::Dependency(_) => false,
        };
        if location {
            self.editing_body
                .as_mut()
                .expect("active body observation")
                .source_location_defaults
                .insert(call);
        }
    }
}
