use std::rc::Rc;

use hir::{
    DefaultCallableDeclarationV1 as Declaration, EditingCallContext,
    EditingCallHierarchyDeclaration as Node, EditingCallHierarchyEdge as Edge, EditingValue as V,
    ImportedCallableSource, SemanticCallable as Callable, SourceContextSubject,
};
use scoop_identity::{CallableTemplateOrigin, SourceIdentity};

use super::*;

impl DeclarationAnalysis {
    pub fn call_hierarchy_body(
        &mut self,
        origin: &hir::SemanticSourceLocation,
        check: Rc<dyn Fn() -> bool>,
    ) -> Result<Option<hir::EditingSourceBody>, EditingCancelled> {
        if let Some(function) = self
            .functions()
            .iter()
            .find(|function| function.name_origin == *origin)
        {
            return Ok(self
                .source_bodies()
                .into_iter()
                .find(|body| body.id == function.function.into()));
        }
        self.source_body_at(&origin.source, origin.span, move || check())
    }

    pub fn call_hierarchy_at(
        &mut self,
        source: &SourceIdentity,
        span: ast::Span,
        check: Rc<dyn Fn() -> bool>,
    ) -> Result<Vec<Node>, EditingCancelled> {
        if check() {
            return Err(EditingCancelled);
        }
        if let Some(function) = self.functions().iter().find(|function| {
            &function.name_origin.source == source && function.name_origin.span == span
        }) {
            return Ok(self
                .call_hierarchy_node(Callable::Current(function.function))
                .into_iter()
                .collect());
        }
        let cancellation = check.clone();
        let Some(body) = self.source_body_at(source, span, move || cancellation())? else {
            return Ok(Vec::new());
        };
        let cancellation = check.clone();
        if let V::Known(facts) = self.source_body_facts(body.id, move || cancellation())?
            && let Some(function) = facts
                .local_functions
                .iter()
                .find(|function| function.name_origin.span == span)
        {
            let callable = Callable::Current(function.function);
            return Ok(self.call_hierarchy_node(callable).into_iter().collect());
        }
        let V::Known(references) = self.callable_references(body.id, move || check())? else {
            return Ok(Vec::new());
        };
        let targets = references
            .iter()
            .filter(|reference| reference.span == span)
            .filter_map(|reference| reference.target.callable())
            .collect::<Vec<_>>();
        Ok(targets
            .into_iter()
            .filter_map(|target| self.call_hierarchy_node(target))
            .collect())
    }

    pub fn call_hierarchy_calls(
        &mut self,
        body: &hir::EditingSourceBody,
        check: Rc<dyn Fn() -> bool>,
    ) -> Result<Vec<Edge>, EditingCancelled> {
        let cancellation = check.clone();
        let V::Known(calls) = self.calls(body.id, move || cancellation())? else {
            return Ok(Vec::new());
        };
        let calls = calls
            .iter()
            .filter_map(|call| {
                let EditingCallContext::Body { source, .. } = &call.context else {
                    return None;
                };
                Some((*source, call.callee?.callable()?, call.callee_span?))
            })
            .collect::<Vec<_>>();
        let mut edges = Vec::new();
        for (context, callee, span) in calls {
            if check() {
                return Err(EditingCancelled);
            }
            let context = &self.lowerer.source_contexts[context];
            let SourceContextSubject::Function(caller) = *context.subject() else {
                continue;
            };
            let Some(caller) = self.call_hierarchy_node(Callable::Current(caller)) else {
                continue;
            };
            let Some(callee) = self.call_hierarchy_node(callee) else {
                continue;
            };
            edges.push(Edge {
                caller,
                callee,
                site: hir::SemanticSourceLocation {
                    source: context.source().clone(),
                    span,
                },
            });
        }
        Ok(edges)
    }

    fn call_hierarchy_node(&self, callable: Callable) -> Option<Node> {
        let origin = self.lowerer.editing_callable_origin(callable)?;
        let (name, method) = match callable {
            Callable::Current(function) => {
                if !matches!(self.signature(function), V::Known(_)) {
                    return None;
                }
                (
                    self.lowerer.source_function_name(function).to_owned(),
                    self.lowerer.functions[function].method.is_some(),
                )
            }
            Callable::Imported(declaration) => {
                let id = match declaration {
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
                (
                    declaration.name().to_owned(),
                    matches!(
                        declaration.interface().owner(),
                        hir::PublicDeclarationOwnerV1::Nominal(_)
                    ),
                )
            }
            Callable::CurrentGetter(_) | Callable::CurrentSetter(_) => return None,
        };
        Some(Node {
            name,
            method,
            origin,
        })
    }
}
