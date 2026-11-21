use super::*;
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
            Some(expected) => match self.describe_body_type(body, context, expected).ok()? {
                V::Known(description) => Some(description.name),
                V::Unavailable(_) => return None,
            },
            None => None,
        };
        Some((key, syntax))
    }

    pub fn is_source_location_call(&self, target: hir::EditingCallee) -> bool {
        use hir::{DefaultCallableDeclarationV1 as D, ImportedCallableSource, SemanticCallable};
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
}
