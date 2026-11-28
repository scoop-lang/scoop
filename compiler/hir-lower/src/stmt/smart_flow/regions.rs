use super::*;

type PredicateExits<F> = (Option<F>, Option<F>, Exits<F>);

impl<A: LocalAnalysis> BodyFlow<'_, A> {
    pub(super) fn try_region(&mut self, incoming: A::Facts, region: &hir::Try) -> Exits<A::Facts> {
        let mut exits = self.statements(incoming.clone(), &region.body);
        if let Some(thrown) = exits.get(&Outcome::Throw).cloned() {
            for catch in &region.catches {
                let mut caught = thrown.clone();
                self.analysis
                    .write(self.state, &mut caught, catch.local, None, None);
                let next = self.statements(caught, &catch.body);
                self.merge(&mut exits, next);
            }
        }
        if let Some(finally) = &region.finally_body {
            let mut result = BTreeMap::new();
            for (outcome, facts) in exits {
                let final_exits = self.statements(facts, finally);
                let final_exits = self.retarget(final_exits, outcome);
                self.merge(&mut result, final_exits);
            }
            result
        } else {
            exits
        }
    }

    pub(super) fn when(&mut self, incoming: A::Facts, when: &hir::When) -> Exits<A::Facts> {
        let mut exits = match when.subject() {
            Some(subject) => self.expression(incoming, subject),
            None => Self::normal(incoming),
        };
        let mut next = exits.remove(&Outcome::Fallthrough);
        for arm in when.arms() {
            let Some(facts) = next.take() else { break };
            let mut matched = match arm.condition {
                hir::WhenConditionRef::Predicate(predicate) => {
                    let (yes, no, abrupt) = self.predicate(facts, predicate);
                    self.merge(&mut exits, abrupt);
                    next = no;
                    yes
                }
                hir::WhenConditionRef::Case(pattern) => {
                    if !crate::patterns::is_irrefutable(pattern) {
                        next = Some(facts.clone());
                    }
                    let mut facts = facts;
                    self.record_pattern(&mut facts, pattern, None);
                    Some(facts)
                }
                hir::WhenConditionRef::Always => Some(facts),
            };
            if let Some(guard) = arm.guard
                && let Some(facts) = matched.take()
            {
                let (yes, no, abrupt) = self.predicate(facts, guard);
                self.merge(&mut exits, abrupt);
                next = match (next, no) {
                    (Some(left), Some(right)) => Some(self.analysis.join(self.state, left, right)),
                    (left, right) => left.or(right),
                };
                matched = yes;
            }
            if let Some(facts) = matched {
                let body = self.statements(facts, arm.body);
                self.merge(&mut exits, body);
            }
        }
        if let Some(facts) = next {
            match when.fallback() {
                hir::WhenFallbackRef::Else(body) => {
                    let body = self.statements(facts, body);
                    self.merge(&mut exits, body);
                }
                hir::WhenFallbackRef::Fallthrough => {
                    self.insert(&mut exits, Outcome::Fallthrough, facts)
                }
                hir::WhenFallbackRef::Impossible(_) => return exits,
            }
        }
        exits
    }

    fn predicate(
        &mut self,
        incoming: A::Facts,
        predicate: &hir::WhenGuard,
    ) -> PredicateExits<A::Facts> {
        let mut exits = self.statements(incoming, &predicate.setup);
        if let Some(facts) = exits.remove(&Outcome::Fallthrough) {
            let expression = self.expression(facts, &predicate.condition);
            self.merge(&mut exits, expression);
        }
        let normal = exits.remove(&Outcome::Fallthrough);
        let yes = normal.clone().and_then(|facts| {
            self.analysis
                .branch(self.state, facts, &predicate.condition, true)
        });
        let no = normal.and_then(|facts| {
            self.analysis
                .branch(self.state, facts, &predicate.condition, false)
        });
        (yes, no, exits)
    }
}
