use super::*;

impl<A: LocalAnalysis> BodyFlow<'_, A> {
    pub(super) fn loop_region(
        &mut self,
        incoming: A::Facts,
        kind: hir::LoopKind,
        target: hir::LoopId,
        setup: &[hir::Statement],
        cond: &hir::Expr,
        body: &[hir::Statement],
    ) -> Exits<A::Facts> {
        let check_reads = self.check_reads;
        self.check_reads = false;
        let mut header = incoming.clone();
        loop {
            let (_, backedge) =
                self.loop_iteration(header.clone(), kind, target, setup, cond, body);
            let next = backedge.map_or_else(
                || incoming.clone(),
                |backedge| self.analysis.join(self.state, incoming.clone(), backedge),
            );
            if next == header {
                break;
            }
            header = next;
        }
        self.check_reads = check_reads;
        self.loop_iteration(header, kind, target, setup, cond, body)
            .0
    }

    fn loop_iteration(
        &mut self,
        header: A::Facts,
        kind: hir::LoopKind,
        target: hir::LoopId,
        setup: &[hir::Statement],
        cond: &hir::Expr,
        body: &[hir::Statement],
    ) -> (Exits<A::Facts>, Option<A::Facts>) {
        let mut exits = BTreeMap::new();
        let mut condition_entry = Some(header.clone());
        if kind == hir::LoopKind::DoWhile {
            let mut body_exits = self.statements(header, body);
            condition_entry = self.take_backedge(&mut body_exits, target);
            self.merge(&mut exits, body_exits);
        }
        let mut backedge = None;
        if let Some(facts) = condition_entry {
            let mut condition = self.condition_region(facts, target, setup, cond);
            let normal = condition.remove(&Outcome::Fallthrough);
            self.merge(&mut exits, condition);
            if let Some(facts) = normal {
                if let Some(no) = self.analysis.branch(self.state, facts.clone(), cond, false) {
                    self.insert(&mut exits, Outcome::Fallthrough, no);
                }
                if let Some(yes) = self.analysis.branch(self.state, facts, cond, true) {
                    if kind == hir::LoopKind::While {
                        let mut body_exits = self.statements(yes, body);
                        backedge = self.take_backedge(&mut body_exits, target);
                        self.merge(&mut exits, body_exits);
                    } else {
                        backedge = Some(yes);
                    }
                }
            }
        }
        if let Some(breaks) = exits.remove(&Outcome::Break(target)) {
            self.insert(&mut exits, Outcome::Fallthrough, breaks);
        }
        (exits, backedge)
    }

    fn condition_region(
        &mut self,
        incoming: A::Facts,
        target: hir::LoopId,
        setup: &[hir::Statement],
        cond: &hir::Expr,
    ) -> Exits<A::Facts> {
        let check_reads = self.check_reads;
        self.check_reads = false;
        let mut header = incoming.clone();
        loop {
            let mut exits = self.condition_iteration(header.clone(), setup, cond);
            let next = exits.remove(&Outcome::Continue(target)).map_or_else(
                || incoming.clone(),
                |facts| self.analysis.join(self.state, incoming.clone(), facts),
            );
            if next == header {
                break;
            }
            header = next;
        }
        self.check_reads = check_reads;
        let mut exits = self.condition_iteration(header, setup, cond);
        exits.remove(&Outcome::Continue(target));
        exits
    }

    fn condition_iteration(
        &mut self,
        incoming: A::Facts,
        setup: &[hir::Statement],
        cond: &hir::Expr,
    ) -> Exits<A::Facts> {
        let mut exits = self.statements(incoming, setup);
        if let Some(facts) = exits.remove(&Outcome::Fallthrough) {
            let next = self.expression(facts, cond);
            self.merge(&mut exits, next);
        }
        exits
    }

    fn take_backedge(
        &mut self,
        exits: &mut Exits<A::Facts>,
        target: hir::LoopId,
    ) -> Option<A::Facts> {
        let normal = exits.remove(&Outcome::Fallthrough);
        let continues = exits.remove(&Outcome::Continue(target));
        self.merge_optional(normal, continues)
    }

    fn merge_optional(
        &mut self,
        left: Option<A::Facts>,
        right: Option<A::Facts>,
    ) -> Option<A::Facts> {
        match (left, right) {
            (Some(left), Some(right)) => Some(self.analysis.join(self.state, left, right)),
            (left, right) => left.or(right),
        }
    }
}
