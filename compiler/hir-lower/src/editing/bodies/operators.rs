use super::*;

impl Lowerer {
    /// Associate one ordinary operator call with its original source token.
    pub(crate) fn with_operator_source<T>(
        &mut self,
        call: ast::Span,
        source: Option<ast::Span>,
        operation: impl FnOnce(&mut Self) -> T,
    ) -> T {
        if !self.observes_current_body() {
            return operation(self);
        }
        let body = self.editing_body.as_mut().expect("active query");
        let names = &mut body.callable_names;
        let previous = match source {
            Some(source) => {
                body.operator_names.insert(source);
                names.insert(call, source)
            }
            None => names.remove(&call),
        };
        let result = operation(self);
        let names = &mut self
            .editing_body
            .as_mut()
            .expect("active query")
            .callable_names;
        match previous {
            Some(source) => {
                names.insert(call, source);
            }
            None => {
                names.remove(&call);
            }
        }
        result
    }
}
