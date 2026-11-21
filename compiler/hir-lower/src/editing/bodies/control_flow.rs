use super::*;

impl DeclarationAnalysis {
    pub fn control_flow_target(
        &mut self,
        body: &hir::EditingSourceBody,
        span: ast::Span,
        is_cancelled: impl Fn() -> bool + 'static,
    ) -> Result<Option<hir::SemanticSourceLocation>, EditingCancelled> {
        self.source_body_facts(body.id, is_cancelled)?;
        let Some(V::Known(observed)) = self.body_facts.get(&body.id) else {
            return Ok(None);
        };
        Ok(observed
            .control_targets
            .get(&span)
            .map(|&span| hir::SemanticSourceLocation {
                source: body.origin.source.clone(),
                span,
            }))
    }
}

impl Lowerer {
    pub(crate) fn observe_literal_target(&mut self, origin: ast::Span) {
        if self.observes_current_body()
            && let Some(context) = self.current_source_context
        {
            self.editing_body
                .as_mut()
                .expect("active body observation")
                .literal_targets
                .insert(context, origin);
        }
    }

    pub(crate) fn observe_return_target(&mut self, span: ast::Span, label: Option<&ast::Ident>) {
        if !self.observes_current_body() {
            return;
        }
        let Some(context) = self.current_source_context else {
            return;
        };
        let target = match self.source_contexts[context].subject() {
            hir::SourceContextSubject::Function(function) => self
                .editing_callable_origin(hir::SemanticCallable::Current(*function))
                .map(|origin| origin.span),
            hir::SourceContextSubject::LexicalCallable { .. } => self
                .editing_body
                .as_ref()
                .and_then(|body| body.literal_targets.get(&context))
                .copied(),
            _ => None,
        };
        if let Some(target) = target {
            self.observe_control_target(ast::Span::new(span.start, span.start + 6), label, target);
        }
    }

    pub(crate) fn observe_control_target(
        &mut self,
        keyword: ast::Span,
        label: Option<&ast::Ident>,
        target: ast::Span,
    ) {
        if !self.observes_current_body() {
            return;
        }
        let targets = &mut self
            .editing_body
            .as_mut()
            .expect("active body observation")
            .control_targets;
        targets.insert(keyword, target);
        if let Some(label) = label {
            targets.insert(label.span, target);
        }
    }
}
