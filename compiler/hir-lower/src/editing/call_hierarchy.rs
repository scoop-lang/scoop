use std::rc::Rc;

use hir::{
    EditingCallContext, EditingCallHierarchyDeclaration as Node, EditingCallHierarchyEdge as Edge,
    EditingCallHierarchyKind as Kind, EditingValue as V, SemanticCallable as Callable,
    SourceContextSubject,
};
use scoop_identity::SourceIdentity;

use super::*;

mod nodes;

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
                .call_hierarchy_node(Callable::Current(function.function).into())
                .into_iter()
                .collect());
        }
        if let Some(property) = self.property_declaration_at(source, span)
            && let Some(node) = self.call_hierarchy_property(
                hir::SemanticProperty::Current(property),
                hir::EditingPropertyAccess::ReadWrite,
            )
        {
            return Ok(vec![node]);
        }
        let cancellation = check.clone();
        let Some(body) = self.source_body_at(source, span, move || cancellation())? else {
            return Ok(self
                .call_hierarchy_constructor_at(source, span)
                .into_iter()
                .collect());
        };
        let cancellation = check.clone();
        if let V::Known(facts) = self.source_body_facts(body.id, move || cancellation())?
            && let Some(function) = facts
                .local_functions
                .iter()
                .find(|function| function.name_origin.span == span)
        {
            let callable = Callable::Current(function.function);
            return Ok(self
                .call_hierarchy_node(callable.into())
                .into_iter()
                .collect());
        }
        if let Some(V::Known(observed)) = self.body_facts.get(&body.id) {
            for call in &observed.calls {
                if let Some((name, target)) = observed.literal_value_calls.get(&call.span)
                    && *name == span
                    && let Some(node) = self.call_hierarchy_node(*target)
                {
                    return Ok(vec![node]);
                }
            }
            if let V::Known(facts) = &observed.facts {
                if let Some(node) = observed
                    .calls
                    .iter()
                    .filter_map(|call| value_binding(call, facts))
                    .find_map(|(site, node)| {
                        (site == span || node.origin.span == span && &node.origin.source == source)
                            .then_some(node)
                    })
                {
                    return Ok(vec![node]);
                }
                if let Some(node) = facts
                    .properties
                    .iter()
                    .filter(|property| property.span == span)
                    .find_map(|property| {
                        self.call_hierarchy_property(property.target, property.access)
                    })
                {
                    return Ok(vec![node]);
                }
            }
        }
        let V::Known(references) = self.callable_references(body.id, move || check())? else {
            return Ok(Vec::new());
        };
        let targets = references
            .iter()
            .filter(|reference| reference.span == span)
            .map(|reference| reference.target)
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
        self.source_body_facts(body.id, move || cancellation())?;
        let Some(V::Known(observed)) = self.body_facts.get(&body.id) else {
            return Ok(Vec::new());
        };
        let mut edges = Vec::new();
        let mut record = |context: hir::SourceContextId, target: Option<Node>, span| {
            let Some(callee) = target else { return };
            let context = &self.lowerer.source_contexts[context];
            let caller = match *context.subject() {
                SourceContextSubject::Function(function) => self
                    .call_hierarchy_node(Callable::Current(function).into())
                    .or_else(|| self.call_hierarchy_owner(body)),
                SourceContextSubject::LexicalCallable { .. } => None,
                _ => self.call_hierarchy_owner(body),
            };
            if let Some(caller) = caller {
                edges.push(Edge {
                    caller,
                    callee,
                    site: hir::SemanticSourceLocation {
                        source: context.source().clone(),
                        span,
                    },
                });
            }
        };
        for call in &observed.calls {
            if check() {
                return Err(EditingCancelled);
            }
            let EditingCallContext::Body { source, .. } = call.context else {
                continue;
            };
            if let Some(&(site, target)) = observed.literal_value_calls.get(&call.span) {
                record(source, self.call_hierarchy_node(target), site);
            } else if let Some((target, site)) = call.callee.zip(call.callee_span) {
                record(source, self.call_hierarchy_node(target), site);
            } else if let V::Known(facts) = &observed.facts
                && let Some((site, node)) = value_binding(call, facts)
            {
                record(source, Some(node), site);
            }
        }
        if let V::Known(facts) = &observed.facts {
            for reference in &facts.callables {
                if check() {
                    return Err(EditingCancelled);
                }
                if let Some(context) = reference.operator_context {
                    record(
                        context,
                        self.call_hierarchy_node(reference.target),
                        reference.span,
                    );
                }
            }
            for property in &facts.properties {
                if check() {
                    return Err(EditingCancelled);
                }
                record(
                    property.context,
                    self.call_hierarchy_property(property.target, property.access),
                    property.span,
                );
            }
        }
        Ok(edges)
    }

    pub fn call_hierarchy_owner(&self, body: &hir::EditingSourceBody) -> Option<Node> {
        self.call_hierarchy_node(body.callee()?)
    }
}

fn value_binding(
    call: &hir::EditingAppliedCall,
    facts: &hir::EditingBodyFacts,
) -> Option<(ast::Span, Node)> {
    if call.callee.is_some() {
        return None;
    }
    let site = call.callee_span?;
    let local = facts
        .locals
        .iter()
        .find(|local| local.span == site && local.role == hir::EditingLocalRole::Read)?;
    Some((
        site,
        Node {
            name: call.name.clone()?,
            kind: Kind::Variable,
            origin: local.reference.definition.clone(),
        },
    ))
}
