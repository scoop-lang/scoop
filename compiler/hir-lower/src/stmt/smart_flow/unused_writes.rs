//! A read consumes every source write that can reach it without another write.
use super::*;
use std::collections::HashMap;

#[derive(Clone, Default, Eq, PartialEq)]
struct ReachingWrites {
    pending: imbl::HashMap<hir::LocalId, imbl::HashSet<Span>>,
    exposed: imbl::HashSet<hir::LocalId>,
}

#[derive(Default)]
struct UnusedWrites {
    used: HashMap<(hir::LocalId, Span), bool>,
}

impl Lowerer {
    pub(crate) fn record_local_write_site(&mut self, local: hir::LocalId, site: Span, name: Span) {
        let local = &self.locals[local];
        if name.start < name.end && !local.name.starts_with('_') {
            self.local_write_sites.insert((local.binding, site), name);
        }
    }

    pub(crate) fn record_assignment_site(
        &mut self,
        place: &ast::PlaceExpr,
        kind: &hir::StatementKind,
        site: Span,
    ) {
        if let (
            ast::PlaceExpr::Name(name),
            hir::StatementKind::Assign {
                target: hir::AssignTarget::Local(local),
                ..
            },
        ) = (place, kind)
        {
            self.record_local_write_site(*local, site, name.span);
        }
    }

    pub(crate) fn warn_unused_local_writes<'a>(
        &mut self,
        statements: &[hir::Statement],
        tail: impl IntoIterator<Item = &'a hir::Expr>,
        errors_before: usize,
    ) {
        if self.diagnostics.len() != errors_before || self.body_cancelled() {
            return;
        }
        let mut analysis = UnusedWrites::default();
        let mut entry = ReachingWrites::default();
        let mut has_bindings = false;
        for (id, local) in self.locals.iter() {
            if let hir::LocalValueDefinitionSite::Source(origin) = local.definition
                && self
                    .local_write_sites
                    .contains_key(&(local.binding, origin.span))
            {
                has_bindings = true;
                if matches!(
                    local.selector,
                    scoop_identity::LocalValueSelector::Parameter { .. }
                ) {
                    analysis.define(&mut entry, id, origin.span);
                }
            }
        }
        if !has_bindings {
            return;
        }
        let mut flow = BodyFlow {
            state: self,
            analysis,
            check_reads: true,
        };
        let mut exits = flow.statements(entry, statements);
        for value in tail {
            let Some(facts) = exits.remove(&Outcome::Fallthrough) else {
                break;
            };
            exits = flow.expression(facts, value);
        }
        let mut unused: Vec<_> = flow
            .analysis
            .used
            .into_iter()
            .filter_map(|((local, span), used)| (!used).then_some((local, span)))
            .collect();
        unused.sort_by_key(|(_, span)| (span.start, span.end));
        for (local, span) in unused {
            let local = &self.locals[local];
            let hir::LocalValueDefinitionSite::Source(origin) = local.definition else {
                continue;
            };
            self.warnings.push(ast::Diagnostic::warning_at_file(
                scoop_source::DiagnosticCode::W0801,
                origin.file as usize,
                span,
                format!("value assigned to `{}` is never read", local.name),
            ));
        }
    }
}

impl UnusedWrites {
    fn define(&mut self, facts: &mut ReachingWrites, local: hir::LocalId, span: Span) {
        let used = self.used.entry((local, span)).or_default();
        if facts.exposed.contains(&local) {
            *used = true;
        } else {
            facts.pending.insert(local, imbl::hashset![span]);
        }
    }
}

impl LocalAnalysis for UnusedWrites {
    type Facts = ReachingWrites;

    fn join(
        &mut self,
        _: &mut Lowerer,
        left: ReachingWrites,
        right: ReachingWrites,
    ) -> ReachingWrites {
        ReachingWrites {
            pending: left
                .pending
                .union_with(right.pending, |left, right| left.union(right)),
            exposed: left.exposed.union(right.exposed),
        }
    }

    fn branch(
        &mut self,
        _: &mut Lowerer,
        facts: ReachingWrites,
        value: &hir::Expr,
        outcome: bool,
    ) -> Option<ReachingWrites> {
        match value.kind {
            hir::ExprKind::BoolLiteral(value) if value != outcome => None,
            _ => Some(facts),
        }
    }

    fn read(
        &mut self,
        state: &mut Lowerer,
        facts: &mut ReachingWrites,
        value: &hir::Expr,
        _: bool,
    ) {
        let local = match value.kind {
            hir::ExprKind::Local(local) => {
                // Ordinary when predicates can refer to the saved subject via
                // the declaration name. This is not a source use of that name.
                if matches!(state.locals[local].definition, hir::LocalValueDefinitionSite::Source(origin) if origin.span == value.span)
                {
                    return;
                }
                local
            }
            hir::ExprKind::AddressOf(hir::Place::Local(local)) => {
                facts.exposed.insert(local);
                local
            }
            _ => return,
        };
        if let Some(writes) = facts.pending.remove(&local) {
            for span in writes {
                self.used.insert((local, span), true);
            }
        }
    }

    fn write(
        &mut self,
        state: &mut Lowerer,
        facts: &mut ReachingWrites,
        local: hir::LocalId,
        _: Option<&hir::Expr>,
        assignment: Option<Span>,
    ) {
        let definition = &state.locals[local];
        let hir::LocalValueDefinitionSite::Source(origin) = definition.definition else {
            return;
        };
        if let Some(&span) = state
            .local_write_sites
            .get(&(definition.binding, assignment.unwrap_or(origin.span)))
        {
            self.define(facts, local, span);
        }
    }
}
