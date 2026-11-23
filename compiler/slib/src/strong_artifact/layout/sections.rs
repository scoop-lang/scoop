use scoop_hir::{
    CoreBootstrapInterfaceSectionV1, CrossConeHirInterfaceSectionV1,
    CrossConeTypeSemanticsSectionV1,
};
use scoop_identity::ConeIdentity;
use scoop_lir::{ConeLirFoundation, CrossConeLayoutAbiSectionV1, CrossConeLirBridgeSectionV1};
use scoop_mir::{
    CoreBootstrapBridgeSectionV1, CrossConeMirBridgeSectionV1, CrossConeMirTypeBridgeSectionV1,
};

use super::CrossConeLayoutArtifactWriteError;
use crate::strong_artifact::{LayerAssembly, StrongArtifactSectionV1, build_section};
use crate::{
    CodeFingerprintDataV2, LinkIdentityClosureSectionV1, MemberPurposeSet, MetadataLocation,
    hir_core_bootstrap_interface_capability, hir_cross_cone_interface_capability,
    hir_cross_cone_type_semantics_capability, hir_identity_foundation_capability,
    lir_cone_production_capability, lir_cross_cone_layout_abi_capability,
    lir_cross_cone_param_free_bridge_capability, lir_identity_foundation_capability,
    lir_link_identity_closure_capability, mir_core_bootstrap_bridge_capability,
    mir_cross_cone_param_free_bridge_capability, mir_cross_cone_type_bridge_capability,
    mir_identity_foundation_capability,
};

pub struct CrossConeLayoutMetadataInputV1<'section, 'ir> {
    pub hir_foundation: &'section scoop_hir::CanonicalHirFoundation,
    pub hir_production: &'section CoreBootstrapInterfaceSectionV1,
    pub hir_cross_cone: CrossConeHirInterfaceSectionV1,
    pub hir_type_semantics: &'section CrossConeTypeSemanticsSectionV1,
    pub mir_foundation: &'section scoop_mir::CanonicalMirFoundation,
    pub mir_production: &'section CoreBootstrapBridgeSectionV1,
    pub mir_cross_cone: &'section CrossConeMirBridgeSectionV1,
    pub mir_type_bridge: &'section CrossConeMirTypeBridgeSectionV1<'ir>,
    pub lir_cross_cone: &'section CrossConeLirBridgeSectionV1,
    pub lir_layout_abi: &'section CrossConeLayoutAbiSectionV1<'ir>,
}

pub(super) struct LayoutMetadataAssembly {
    pub hir: crate::SlibMember,
    pub mir: crate::SlibMember,
    pub lir: crate::SlibMember,
    pub fingerprints: crate::SemanticFingerprintRecord,
}

/// Encoded semantic publication data, independent of dependency readers and IR.
pub struct PreparedCrossConeLayoutMetadataV1 {
    hir: LayerAssembly,
    mir: LayerAssembly,
    ordinary: crate::MetadataSection,
    layout_interface: crate::MetadataSection,
    layout: crate::MetadataSection,
}

impl PreparedCrossConeLayoutMetadataV1 {
    pub fn new(
        mut input: CrossConeLayoutMetadataInputV1<'_, '_>,
    ) -> Result<Self, CrossConeLayoutArtifactWriteError> {
        let hir_cross_cone = input
            .hir_cross_cone
            .index_for_wire()
            .map_err(CrossConeLayoutArtifactWriteError::HirInterfaceIndex)?;
        let hir = LayerAssembly::new(
            MetadataLocation::Hir,
            vec![
                build_section(
                    StrongArtifactSectionV1::HirFoundation,
                    MetadataLocation::Hir,
                    hir_identity_foundation_capability(),
                    MemberPurposeSet::COMPILE_AND_LINK,
                    input.hir_foundation,
                )?,
                build_section(
                    StrongArtifactSectionV1::HirProduction,
                    MetadataLocation::Hir,
                    hir_core_bootstrap_interface_capability(),
                    MemberPurposeSet::COMPILE,
                    input.hir_production,
                )?,
                build_section(
                    StrongArtifactSectionV1::HirCrossConeInterface,
                    MetadataLocation::Hir,
                    hir_cross_cone_interface_capability(),
                    MemberPurposeSet::COMPILE,
                    &hir_cross_cone,
                )?,
                build_section(
                    StrongArtifactSectionV1::HirCrossConeTypeSemantics,
                    MetadataLocation::Hir,
                    hir_cross_cone_type_semantics_capability(),
                    MemberPurposeSet::COMPILE,
                    input.hir_type_semantics,
                )?,
            ],
        )?;
        drop(hir_cross_cone);
        drop(input.hir_cross_cone);
        let mir = LayerAssembly::new(
            MetadataLocation::Mir,
            vec![
                build_section(
                    StrongArtifactSectionV1::MirFoundation,
                    MetadataLocation::Mir,
                    mir_identity_foundation_capability(),
                    MemberPurposeSet::COMPILE_AND_LINK,
                    input.mir_foundation,
                )?,
                build_section(
                    StrongArtifactSectionV1::MirProduction,
                    MetadataLocation::Mir,
                    mir_core_bootstrap_bridge_capability(),
                    MemberPurposeSet::COMPILE,
                    input.mir_production,
                )?,
                build_section(
                    StrongArtifactSectionV1::MirCrossConeBridge,
                    MetadataLocation::Mir,
                    mir_cross_cone_param_free_bridge_capability(),
                    MemberPurposeSet::COMPILE,
                    input.mir_cross_cone,
                )?,
                build_section(
                    StrongArtifactSectionV1::MirCrossConeTypeBridge,
                    MetadataLocation::Mir,
                    mir_cross_cone_type_bridge_capability(),
                    MemberPurposeSet::COMPILE,
                    input.mir_type_bridge,
                )?,
            ],
        )?;
        let ordinary = build_section(
            StrongArtifactSectionV1::LirCrossConeBridge,
            MetadataLocation::Lir,
            lir_cross_cone_param_free_bridge_capability(),
            MemberPurposeSet::COMPILE_AND_LINK,
            input.lir_cross_cone,
        )?;
        let layout_interface = build_section(
            StrongArtifactSectionV1::LirCrossConeLayoutAbi,
            MetadataLocation::Lir,
            lir_cross_cone_layout_abi_capability(),
            MemberPurposeSet::COMPILE_AND_LINK,
            input.lir_layout_abi.exports(),
        )?;
        let layout = build_section(
            StrongArtifactSectionV1::LirCrossConeLayoutAbi,
            MetadataLocation::Lir,
            lir_cross_cone_layout_abi_capability(),
            MemberPurposeSet::COMPILE_AND_LINK,
            input.lir_layout_abi,
        )?;
        Ok(Self {
            hir,
            mir,
            ordinary,
            layout_interface,
            layout,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn assemble(
        self,
        cone: ConeIdentity,
        lir_foundation: &ConeLirFoundation,
        code: &CodeFingerprintDataV2,
        link_identity_closure: &LinkIdentityClosureSectionV1,
        link_support: &crate::LirLinkSupportSectionV1,
        compatibility: &crate::CompatibilityRecord,
        direct_dependencies: &[crate::DependencyRecord],
    ) -> Result<LayoutMetadataAssembly, CrossConeLayoutArtifactWriteError> {
        let Self {
            hir,
            mir,
            ordinary,
            layout_interface,
            layout,
        } = self;
        let lir_semantic_sections = [ordinary, layout_interface];
        let fingerprints = crate::SemanticFingerprintRecord::from_metadata_sections(
            compatibility,
            direct_dependencies,
            hir.sections(),
            mir.sections(),
            &lir_semantic_sections,
        )
        .map_err(CrossConeLayoutArtifactWriteError::SemanticFingerprints)?;
        let [ordinary, layout_interface] = lir_semantic_sections;
        drop(layout_interface);
        let hir = hir.into_member(cone)?;
        let mir = mir.into_member(cone)?;
        let lir = LayerAssembly::new(
            MetadataLocation::Lir,
            vec![
                build_section(
                    StrongArtifactSectionV1::LinkSupport,
                    MetadataLocation::Lir,
                    crate::lir_link_support_capability(),
                    MemberPurposeSet::COMPILE_AND_LINK,
                    link_support,
                )?,
                build_section(
                    StrongArtifactSectionV1::LirFoundation,
                    MetadataLocation::Lir,
                    lir_identity_foundation_capability(),
                    MemberPurposeSet::COMPILE_AND_LINK,
                    lir_foundation,
                )?,
                ordinary,
                build_section(
                    StrongArtifactSectionV1::LirProduction,
                    MetadataLocation::Lir,
                    lir_cone_production_capability(),
                    MemberPurposeSet::COMPILE_AND_LINK,
                    code.production().strong_production(),
                )?,
                layout,
                build_section(
                    StrongArtifactSectionV1::LinkIdentityClosure,
                    MetadataLocation::Lir,
                    lir_link_identity_closure_capability(),
                    MemberPurposeSet::LINK,
                    link_identity_closure,
                )?,
            ],
        )?;
        let lir = lir.into_member(cone)?;
        Ok(LayoutMetadataAssembly {
            hir,
            mir,
            lir,
            fingerprints,
        })
    }
}
