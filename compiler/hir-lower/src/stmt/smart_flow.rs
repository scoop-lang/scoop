//! Shared control flow for smart casts during lowering and unused writes in completed bodies.
use super::*;
use crate::expr::SmartCastFacts;
use std::collections::BTreeMap;

mod expressions;
mod loops;
mod operands;
mod regions;
mod statements;
mod type_facts;
mod unused_writes;

type Outcome = flow::ControlOutcome<hir::LoopId>;
type Exits<F> = BTreeMap<Outcome, F>;
pub(crate) type SmartCastExits = Exits<SmartCastFacts>;

trait LocalAnalysis {
    type Facts: Clone + Eq;
    fn join(&mut self, state: &mut Lowerer, left: Self::Facts, right: Self::Facts) -> Self::Facts;
    fn branch(
        &mut self,
        state: &mut Lowerer,
        facts: Self::Facts,
        value: &hir::Expr,
        outcome: bool,
    ) -> Option<Self::Facts>;
    fn read(
        &mut self,
        state: &mut Lowerer,
        facts: &mut Self::Facts,
        value: &hir::Expr,
        check_reads: bool,
    );
    fn write(
        &mut self,
        state: &mut Lowerer,
        facts: &mut Self::Facts,
        local: hir::LocalId,
        value: Option<&hir::Expr>,
        assignment: Option<Span>,
    );
}

struct TypeFacts;

struct BodyFlow<'a, A> {
    analysis: A,
    state: &'a mut Lowerer,
    check_reads: bool,
}

impl Lowerer {
    pub(crate) fn smart_cast_exits(
        &mut self,
        incoming: SmartCastFacts,
        statements: &[hir::Statement],
    ) -> SmartCastExits {
        BodyFlow {
            analysis: TypeFacts,
            state: self,
            check_reads: false,
        }
        .statements(incoming, statements)
    }

    pub(crate) fn advance_smart_casts(
        &mut self,
        incoming: SmartCastFacts,
        statements: &[hir::Statement],
        value: Option<&hir::Expr>,
    ) {
        let mut flow = BodyFlow {
            analysis: TypeFacts,
            state: self,
            check_reads: true,
        };
        let mut exits = flow.statements(incoming.clone(), statements);
        if let Some(value) = value
            && let Some(facts) = exits.remove(&Outcome::Fallthrough)
        {
            let next = flow.expression(facts, value);
            flow.merge(&mut exits, next);
        }
        self.smart_casts = exits.remove(&Outcome::Fallthrough).unwrap_or(incoming);
    }

    pub(crate) fn joined_exit_facts(&mut self, exits: &SmartCastExits) -> Option<SmartCastFacts> {
        let mut facts = exits.values().cloned();
        let mut result = facts.next()?;
        for facts in facts {
            result = self.common_smart_casts(result, facts);
        }
        Some(result)
    }

    pub(crate) fn catch_smart_cast_facts(
        &mut self,
        incoming: SmartCastFacts,
        body: &[hir::Statement],
        value: Option<&hir::Expr>,
    ) -> SmartCastFacts {
        let mut flow = BodyFlow {
            analysis: TypeFacts,
            state: self,
            check_reads: false,
        };
        let mut exits = flow.statements(incoming.clone(), body);
        if let Some(value) = value
            && let Some(facts) = exits.remove(&Outcome::Fallthrough)
        {
            let next = flow.expression(facts, value);
            flow.merge(&mut exits, next);
        }
        exits.remove(&Outcome::Throw).unwrap_or(incoming)
    }

    pub(crate) fn finally_smart_cast_facts<'a>(
        &mut self,
        incoming: SmartCastFacts,
        body: &[hir::Statement],
        catches: impl Iterator<Item = &'a [hir::Statement]>,
    ) -> SmartCastFacts {
        let mut exits = self.smart_cast_exits(incoming.clone(), body);
        if let Some(thrown) = exits.get(&Outcome::Throw).cloned() {
            for body in catches {
                let next = self.smart_cast_exits(thrown.clone(), body);
                BodyFlow {
                    analysis: TypeFacts,
                    state: self,
                    check_reads: false,
                }
                .merge(&mut exits, next);
            }
        }
        self.joined_exit_facts(&exits).unwrap_or(incoming)
    }
}

impl<A: LocalAnalysis> BodyFlow<'_, A> {
    fn normal(facts: A::Facts) -> Exits<A::Facts> {
        BTreeMap::from([(Outcome::Fallthrough, facts)])
    }

    fn merge(&mut self, target: &mut Exits<A::Facts>, source: Exits<A::Facts>) {
        for (outcome, facts) in source {
            self.insert(target, outcome, facts);
        }
    }

    fn insert(&mut self, exits: &mut Exits<A::Facts>, outcome: Outcome, facts: A::Facts) {
        let facts = if let Some(previous) = exits.remove(&outcome) {
            self.analysis.join(self.state, previous, facts)
        } else {
            facts
        };
        exits.insert(outcome, facts);
    }

    fn statements(&mut self, incoming: A::Facts, statements: &[hir::Statement]) -> Exits<A::Facts> {
        let mut exits = Self::normal(incoming);
        for statement in statements {
            let Some(facts) = exits.remove(&Outcome::Fallthrough) else {
                break;
            };
            let next = self.statement(facts, statement);
            self.merge(&mut exits, next);
        }
        exits
    }

    fn retarget(&mut self, mut exits: Exits<A::Facts>, target: Outcome) -> Exits<A::Facts> {
        if let Some(facts) = exits.remove(&Outcome::Fallthrough) {
            self.insert(&mut exits, target, facts);
        }
        exits
    }
}
