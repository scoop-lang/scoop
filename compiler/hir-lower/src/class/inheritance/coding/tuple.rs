use super::*;

impl Lowerer {
    pub(super) fn coding_tuple_function(
        &mut self,
        context: CodingContext,
        target: TypeId,
        span: Span,
        sink: &mut Vec<hir::Statement>,
    ) -> Option<hir::Expr> {
        let stream = self.core_coding_nominal("ValueView")?;
        let stream = self
            .apply_nominal_type(stream, Vec::new())
            .expect("a coding stream is an ordinary interface");
        let function =
            self.lower_synthesized_lambda(false, &[target], stream, span, |state, parameters| {
                let mut statements = Vec::new();
                let value = state.inspect_target(
                    context,
                    target,
                    parameters[0].clone(),
                    span,
                    &mut statements,
                )?;
                Some(crate::stmt::ValueBlock {
                    statements,
                    value: Some(value),
                })
            })?;
        let adapter = self.core_coding_nominal("ValueInspectorFunction")?;
        let adapter = self
            .apply_nominal_type(adapter, vec![target])
            .expect("a function codec has one unconstrained parameter");
        self.construct_inspection_value(adapter, vec![function], span, sink)
    }
}
