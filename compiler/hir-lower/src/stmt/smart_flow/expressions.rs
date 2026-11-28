use super::*;

impl<A: LocalAnalysis> BodyFlow<'_, A> {
    pub(super) fn expression(&mut self, facts: A::Facts, value: &hir::Expr) -> Exits<A::Facts> {
        if let hir::ExprKind::Binary {
            op: hir::BinOp::And | hir::BinOp::Or,
            lhs,
            rhs,
        } = &value.kind
        {
            let hir::ExprKind::Binary { op, .. } = &value.kind else {
                unreachable!()
            };
            return self.short_circuit(facts, *op == hir::BinOp::And, lhs, rhs);
        }
        let mut exits = Self::normal(facts);
        for operand in operands::operands(value) {
            if let Some(facts) = exits.remove(&Outcome::Fallthrough) {
                let next = self.expression(facts, operand);
                self.merge(&mut exits, next);
            }
        }
        let (receiver, captures): (_, &[hir::Capture]) = match value.kind {
            hir::ExprKind::Lambda(id) => (None, &self.state.lambdas[id].captures),
            hir::ExprKind::AnonymousFunction(id) => {
                (None, &self.state.anonymous_functions[id].captures)
            }
            hir::ExprKind::CallableReference(id) => {
                let reference = &self.state.callable_references[id];
                (reference.target.receiver(), &reference.captures)
            }
            _ => (None, &[]),
        };
        let inputs: Vec<_> = receiver
            .into_iter()
            .chain(captures.iter().map(|capture| &capture.source))
            .cloned()
            .collect();
        for input in inputs {
            if let Some(facts) = exits.remove(&Outcome::Fallthrough) {
                let next = self.expression(facts, &input);
                self.merge(&mut exits, next);
            }
        }
        if let Some(mut facts) = exits.remove(&Outcome::Fallthrough) {
            self.analysis
                .read(self.state, &mut facts, value, self.check_reads);
            if self.state.is_nothing_ty(value.ty) {
                self.insert(&mut exits, Outcome::Throw, facts);
            } else {
                if operands::may_throw(value) {
                    self.insert(&mut exits, Outcome::Throw, facts.clone());
                }
                exits.insert(Outcome::Fallthrough, facts);
            }
        }
        exits
    }

    fn short_circuit(
        &mut self,
        facts: A::Facts,
        enter: bool,
        lhs: &hir::Expr,
        rhs: &hir::Expr,
    ) -> Exits<A::Facts> {
        let mut exits = self.expression(facts, lhs);
        if let Some(facts) = exits.remove(&Outcome::Fallthrough) {
            if let Some(short) = self.analysis.branch(self.state, facts.clone(), lhs, !enter) {
                self.insert(&mut exits, Outcome::Fallthrough, short);
            }
            if let Some(right) = self.analysis.branch(self.state, facts, lhs, enter) {
                let next = self.expression(right, rhs);
                self.merge(&mut exits, next);
            }
        }
        exits
    }
}
