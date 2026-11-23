//! Layout-profile assembly from the actual V2 object and semantic products.

use super::*;
use crate::object_production::{BuiltinObjectProductionError as ObjectError, layout as objects};
use scoop_slib as slib;

mod error;
use LayoutArtifactProductionError as Error;
pub use error::LayoutArtifactProductionError;

pub struct CrossConeLayoutArtifactMetadataInputV1<'dependencies> {
    producer: ProducerRecord,
    cone: ConeRecord,
    direct_dependencies: Vec<DependencyRecord>,
    metadata: slib::PreparedCrossConeLayoutMetadataV1,
    lir_cross_cone: scoop_lir::CrossConeLirBridgeSectionV1,
    consumer: scoop_identity::ConeIdentity,
    physical_imports: scoop_lir::CanonicalExternalShapeLinkImportsV1,
    link_support: slib::LirLinkSupportSectionV1,
    native_objects: Vec<slib::UncompressedObjectMember>,
    native_libraries: Vec<scoop_lir::CanonicalNativeLibraryRequirementV1>,
    native_cxx: bool,
    test_directory: Option<slib::TestDirectory>,
    dependencies: Option<&'dependencies slib::CrossConePublicationDependencies>,
}

impl<'dependencies> CrossConeLayoutArtifactMetadataInputV1<'dependencies> {
    /// Encodes completed semantic sections before their readers and IR are released.
    pub fn new(
        ordinary: CrossConeArtifactMetadataInputV1<'_>,
        hir_types: &scoop_hir::CrossConeTypeSemanticsSectionV1,
        mir_types: &scoop_mir::CrossConeMirTypeBridgeSectionV1<'_>,
        lir_layout: &scoop_lir::CrossConeLayoutAbiSectionV1<'_>,
    ) -> Result<Self, Error> {
        let descriptor = ordinary
            .hir_core
            .compiler_protocols()
            .map(|protocols| {
                lir_layout
                    .descriptors()
                    .get(protocols.string_exact_type())
                    .ok_or_else(|| {
                        slib::LinkSupportError("String descriptor missing from producer LIR".into())
                    })
            })
            .transpose()
            .map_err(Error::LinkSupport)?;
        let link_support = slib::LirLinkSupportSectionV1::from_string_descriptor(descriptor)
            .map_err(Error::LinkSupport)?;
        let metadata =
            slib::PreparedCrossConeLayoutMetadataV1::new(slib::CrossConeLayoutMetadataInputV1 {
                hir_foundation: ordinary.hir_foundation,
                hir_production: ordinary.hir_core,
                hir_cross_cone: ordinary.hir_cross_cone,
                hir_type_semantics: hir_types,
                mir_foundation: ordinary.mir_foundation,
                mir_production: ordinary.mir_core,
                mir_cross_cone: ordinary.mir_cross_cone,
                mir_type_bridge: mir_types,
                lir_cross_cone: ordinary.lir_cross_cone,
                lir_layout_abi: lir_layout,
            })
            .map_err(|error| Error::Assembly(Box::new(error)))?;
        Ok(Self {
            producer: ordinary.producer,
            cone: ordinary.cone,
            direct_dependencies: ordinary.direct_dependencies,
            metadata,
            lir_cross_cone: ordinary.lir_cross_cone.clone(),
            consumer: lir_layout.selected().consumer(),
            physical_imports: lir_layout.selected().physical_imports().clone(),
            link_support,
            native_objects: Vec::new(),
            native_libraries: Vec::new(),
            native_cxx: false,
            test_directory: None,
            dependencies: None,
        })
    }

    pub fn with_test_directory(mut self, directory: Option<slib::TestDirectory>) -> Self {
        self.test_directory = directory;
        self
    }

    pub fn with_dependencies(
        mut self,
        dependencies: &'dependencies slib::CrossConePublicationDependencies,
    ) -> Self {
        self.dependencies = Some(dependencies);
        self
    }

    pub fn with_native_objects(mut self, objects: Vec<slib::UncompressedObjectMember>) -> Self {
        self.native_objects = objects;
        self
    }

    pub fn with_native_libraries(
        mut self,
        libraries: Vec<scoop_lir::CanonicalNativeLibraryRequirementV1>,
    ) -> Self {
        self.native_libraries = libraries;
        self
    }

    pub fn with_native_cxx(mut self, cxx: bool) -> Self {
        self.native_cxx = cxx;
        self
    }

    pub fn assemble(
        self,
        emitted: scoop_codegen::EmittedConeObjectSetV2,
        generated: scoop_codegen::EmittedGeneratedCBridgeObjectSetV1,
        dependency_owners: &[slib::CanonicalDefinedLinkSymbolOwnerSetV1],
    ) -> Result<slib::AssembledCrossConeLayoutArtifactV1, Error> {
        let optimization = emitted.optimization;
        let mut prepared = objects::prepare(emitted, generated)?;
        prepared.foundation = prepared
            .foundation
            .with_native_library_requirements(self.native_libraries)
            .with_native_cxx(self.native_cxx);
        let strong = prepared.builtins.strong_relocations().clone();
        let defined =
            slib::CanonicalDefinedLinkSymbolOwnerSetV1::from_verified_strong_closure(&strong)
                .map_err(ObjectError::DefinedSymbols)?;
        let native = scoop_lir::CanonicalNativeExternalRequirementSurfaceV1::from_foundation(
            prepared.target_selection.target(),
            &prepared.foundation,
        )
        .map_err(ObjectError::NativeRequirementSurface)?;
        let ordinary = slib::verify_cross_cone_strong_requirements_v1(
            prepared.target_selection.target(),
            strong,
            dependency_owners,
            &self.lir_cross_cone,
        )
        .map_err(ObjectError::DependencyRequirements)?;
        let shape = slib::verify_external_shape_requirements_v1(
            &ordinary,
            self.consumer,
            &self.physical_imports,
        )
        .map_err(|error| Error::Layout(Box::new(error)))?;
        let undefined = objects::complete_requirements(&prepared, &native, &shape)?;
        let finalized = prepared.finalize(&undefined, &self.cone, self.native_objects)?;
        let code = slib::compute_cross_cone_layout_code_fingerprint_v1(
            finalized.projection.with_optimization(optimization),
            native,
            defined,
            undefined,
            &shape,
            self.link_support,
        )
        .map_err(|error| Error::Code(Box::new(error)))?;
        let artifact = slib::AssembledCrossConeLayoutArtifactV1::write(
            slib::CrossConeLayoutArtifactInputV1::new(
                self.producer,
                self.cone,
                self.direct_dependencies,
                self.metadata,
                &finalized.foundation,
                code,
                finalized.members,
            )
            .with_test_directory(self.test_directory)
            .with_dependencies(self.dependencies),
        )
        .map_err(|error| Error::Assembly(Box::new(error)))?;
        Ok(artifact)
    }
}
