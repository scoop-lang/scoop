//! Physical dependency selection follows actual MIR references.

use super::*;
use LayoutProductionError as Error;
use scoop_lir as lir;
use scoop_mir as mir;
use scoop_slib as slib;

pub(in crate::request::preflight) fn select_lir_dependencies<'a>(
    input: &mir::ConeMirInput,
    initialization: impl IntoIterator<
        Item = (ConeIdentity, scoop_identity::PersistentInitializationUnitId),
    >,
    dependencies: &[&'a slib::PhysicalImportsReplayedCrossConeLayoutSections],
    target: lir::LirTargetProfile,
) -> Result<lir::StrongProductionDependencySelectionV2<'a>, Error> {
    let mut roots = Vec::new();
    let mut physical = Vec::new();
    let exports: Vec<_> = dependencies.iter().map(|d| d.lir_exports()).collect();
    collect_mir_references(input, &exports, &mut roots, &mut physical)?;
    physical.extend(initialization.into_iter().map(|(provider, unit)| {
        (
            provider,
            lir::ExternalStrongShapeSubjectV1::InitializationRegistration(unit),
        )
    }));
    for &id in input.materialization().immortals() {
        let immortal = &input.module().immortals[id];
        if let mir::ImmortalConstSource::Imported(provider) = immortal.source {
            physical.push((
                provider,
                lir::ExternalStrongShapeSubjectV1::ImmortalObject(
                    scoop_identity::PersistentImmortalObjectId::from_key(&immortal.identity)
                        .map_err(Error::ImmortalIdentity)?,
                ),
            ));
        }
    }
    roots.sort_unstable();
    roots.dedup();
    physical.sort_unstable();
    physical.dedup();
    let providers = dependencies
        .iter()
        .map(|dependency| {
            let exports = dependency.lir_exports();
            lir::ShapeLinkProviderV1::try_new(lir::ShapeLinkProviderPartsV1 {
                foundation: dependency.lir_foundation(),
                definitions: dependency.lir_production_data().canonical_definitions(),
                support: lir::ShapeLinkSupportV1::Saved(dependency.lir_production_data()),
                ordinary: dependency.lir_cross_cone_bridge(),
                layouts: exports.layouts(),
                callables: exports.callables(),
                descriptors: exports.descriptors(),
                dispatch: exports.dispatch(),
            })
            .map(|provider| (provider.provider(), provider))
        })
        .collect::<Result<std::collections::HashMap<_, _>, _>>()
        .map_err(Error::Shape)?;
    let imports = physical
        .into_iter()
        .map(|(identity, subject)| {
            let provider = providers
                .get(&identity)
                .ok_or(lir::ShapeLinkError::MissingProvider(identity))?;
            lir::ExternalShapeLinkImportV1::replay(provider, subject, input.module().cone)
        })
        .collect::<Result<Vec<_>, _>>()
        .map_err(Error::Shape)?;
    lir::StrongProductionDependencySelectionV2::try_new(
        input.module().cone,
        target,
        &exports,
        imports,
        &roots,
    )
    .map_err(Error::Selection)
}
