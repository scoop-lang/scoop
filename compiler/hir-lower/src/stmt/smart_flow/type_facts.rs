use super::*;

impl LocalAnalysis for TypeFacts {
    type Facts = SmartCastFacts;

    fn join(
        &mut self,
        state: &mut Lowerer,
        left: SmartCastFacts,
        right: SmartCastFacts,
    ) -> SmartCastFacts {
        state.common_smart_casts(left, right)
    }

    fn branch(
        &mut self,
        state: &mut Lowerer,
        facts: SmartCastFacts,
        value: &hir::Expr,
        outcome: bool,
    ) -> Option<SmartCastFacts> {
        state.refine_flow_condition(facts, value, outcome)
    }

    fn read(
        &mut self,
        state: &mut Lowerer,
        facts: &mut SmartCastFacts,
        value: &hir::Expr,
        check_reads: bool,
    ) {
        check_read(state, facts, value, check_reads);
        if let hir::ExprKind::AddressOf(hir::Place::Local(local)) = value.kind {
            facts.expose_address(state.locals[local].binding);
        }
    }

    fn write(
        &mut self,
        state: &mut Lowerer,
        facts: &mut SmartCastFacts,
        local: hir::LocalId,
        value: Option<&hir::Expr>,
        _: Option<Span>,
    ) {
        if let Some(value) = value {
            state.record_flow_value(facts, local, value);
        }
    }
}

fn check_read(state: &mut Lowerer, facts: &SmartCastFacts, value: &hir::Expr, check_reads: bool) {
    if !check_reads
        || !matches!(
            value.kind,
            hir::ExprKind::Local(_) | hir::ExprKind::Unbox(_)
        )
    {
        return;
    }
    let Some(local) = source_local(value) else {
        return;
    };
    let local = &state.locals[local];
    if !local.mutable {
        return;
    }
    let (binding, declared) = (local.binding, local.ty);
    let name = local.name.clone();
    if state.is_subtype(declared, value.ty)
        || facts
            .get(&binding)
            .is_some_and(|types| types.iter().any(|&ty| state.is_subtype(ty, value.ty)))
    {
        return;
    }
    let message = format!(
        "smart cast of mutable local `{name}` to {} is not valid on every incoming path",
        state.type_name(value.ty)
    );
    if !state
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.span() == Some(value.span) && diagnostic.message == message)
    {
        state.error(scoop_source::DiagnosticCode::E0812, value.span, message);
    }
}

fn source_local(value: &hir::Expr) -> Option<hir::LocalId> {
    match &value.kind {
        hir::ExprKind::Local(local) => Some(*local),
        hir::ExprKind::Unbox(source)
        | hir::ExprKind::Upcast(source)
        | hir::ExprKind::Box(source) => source_local(source),
        _ => None,
    }
}
