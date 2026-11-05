//! Registrations and Context cells selected together with one callable body.

use super::*;

pub(crate) fn emit_callable_metadata_v1<'ctx, D, C, I>(
    context: &'ctx Context,
    llvm: &LlvmModule<'ctx>,
    profile: ValidatedBackendProfile,
    production: &scoop_lir::ConeProductionSection<D, C, I>,
    body: scoop_lir::PersistentCallableBodyId,
) -> Result<(), CodegenError> {
    let callables = emit_strong_callable_registrations_v1(
        context,
        llvm,
        production.registration_production().callables(),
        production.canonical_definitions(),
        profile,
        body,
    )?;
    let mut atoms = callables.context_atoms.clone();
    for registration in callables.registrations() {
        atoms.push(GlobalAtomMaterializationV1::new(
            registration.primary_atom(),
            registration.descriptor(),
        ));
    }
    emit_global_atom_boundaries_v1(llvm, production.canonical_definitions(), atoms)?;
    Ok(())
}

pub(crate) fn bind_callable_gc_map<'ctx, D, C, I>(
    context: &'ctx Context,
    llvm: &LlvmModule<'ctx>,
    production: &scoop_lir::ConeProductionSection<D, C, I>,
    body: scoop_lir::PersistentCallableBodyId,
    expected: &crate::statepoint::LoweredSafepoints,
) -> Result<(), CodegenError> {
    if expected.site_count() == 0 {
        return Ok(());
    }
    let plan = production
        .registration_production()
        .callables()
        .registrations()
        .iter()
        .find(|plan| plan.body() == body)
        .ok_or_else(|| CodegenError(format!("missing callable registration {body}")))?;
    let definition = production
        .canonical_definitions()
        .plan(plan.body_definition_plan())
        .ok_or_else(|| CodegenError(format!("missing callable definition {body}")))?;
    let boundary = definition
        .atom_boundaries()
        .iter()
        .find(|boundary| boundary.atom_role() == scoop_lir::DefinitionAtomRole::Stackmap)
        .ok_or_else(|| CodegenError(format!("missing callable GC atom {body}")))?;
    // The object boundary writer resolves this declaration to the machine table.
    let name = boundary.start().symbol();
    let map = llvm.add_global(context.i8_type(), None, name.as_str());
    map.set_constant(true);
    let descriptor = llvm
        .get_global(plan.symbol().symbol().as_str())
        .ok_or_else(|| CodegenError(format!("missing emitted callable registration {body}")))?;
    let value = descriptor
        .get_initializer()
        .ok_or_else(|| CodegenError(format!("uninitialized callable registration {body}")))?
        .into_struct_value();
    let mut fields = (0..6)
        .map(|index| {
            value
                .get_field_at_index(index)
                .expect("the callable descriptor has six fields")
        })
        .collect::<Vec<_>>();
    fields[5] = map.as_pointer_value().into();
    descriptor.set_initializer(&value.get_type().const_named_struct(&fields));
    Ok(())
}
