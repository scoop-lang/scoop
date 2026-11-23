//! Deterministic archive assembly for the cross-Cone layout strong profile.

use crate::{
    ArtifactCapabilityProfile, ArtifactFingerprint, BootstrapManifest, CanonicalSlibArchive,
    CompatibilityRecord, ConeRecord, CrossConeLayoutCodeDataV1, DependencyRecord,
    FingerprintAvailability, LinkIdentityClosureSectionV1, ProducerRecord,
    SemanticFingerprintRecord, UncompressedObjectMember,
};
use scoop_lir::{ConeLirFoundation, ValidatedLirTargetSelection};

mod error;
pub use error::CrossConeLayoutArtifactWriteError;
mod sections;
pub use sections::{CrossConeLayoutMetadataInputV1, PreparedCrossConeLayoutMetadataV1};
mod odr_reuse;
pub use odr_reuse::CrossConePublicationDependencies;

/// Complete typed input for one `CrossConeLayoutStrong` archive.
///
/// Strong V2, Link closures, object coverage, fingerprints, and the production
/// manifest come from the same completed object production.
pub struct CrossConeLayoutArtifactInputV1<'ir> {
    producer: ProducerRecord,
    cone: ConeRecord,
    direct_dependencies: Vec<DependencyRecord>,
    metadata: PreparedCrossConeLayoutMetadataV1,
    lir_foundation: &'ir ConeLirFoundation,
    layout_code: CrossConeLayoutCodeDataV1,
    link_objects: Vec<UncompressedObjectMember>,
    test_directory: Option<crate::TestDirectory>,
    dependencies: Option<&'ir CrossConePublicationDependencies>,
}

impl<'ir> CrossConeLayoutArtifactInputV1<'ir> {
    pub fn with_test_directory(mut self, directory: Option<crate::TestDirectory>) -> Self {
        self.test_directory = directory;
        self
    }

    pub fn with_dependencies(
        mut self,
        dependencies: Option<&'ir CrossConePublicationDependencies>,
    ) -> Self {
        self.dependencies = dependencies;
        self
    }

    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        producer: ProducerRecord,
        cone: ConeRecord,
        direct_dependencies: Vec<DependencyRecord>,
        metadata: PreparedCrossConeLayoutMetadataV1,
        lir_foundation: &'ir ConeLirFoundation,
        layout_code: CrossConeLayoutCodeDataV1,
        link_objects: Vec<UncompressedObjectMember>,
    ) -> Self {
        Self {
            producer,
            cone,
            direct_dependencies,
            metadata,
            lir_foundation,
            layout_code,
            link_objects,
            test_directory: None,
            dependencies: None,
        }
    }
}

/// Canonical layout-profile bytes and their publication identities.
#[derive(Debug, Eq, PartialEq)]
pub struct AssembledCrossConeLayoutArtifactV1 {
    archive: CanonicalSlibArchive,
    summary: crate::CrossConeArtifactSummary,
}

impl AssembledCrossConeLayoutArtifactV1 {
    pub fn write(
        input: CrossConeLayoutArtifactInputV1<'_>,
    ) -> Result<Self, CrossConeLayoutArtifactWriteError> {
        let CrossConeLayoutArtifactInputV1 {
            producer,
            cone,
            direct_dependencies,
            metadata,
            lir_foundation,
            layout_code,
            mut link_objects,
            test_directory,
            dependencies,
        } = input;
        let identity = cone.identity();
        let (code, link_support) = layout_code.into_parts();

        let target_selection = code.undefined_symbols().selection();
        let compatibility = CompatibilityRecord::new(
            target_selection,
            ArtifactCapabilityProfile::CROSS_CONE_GENERIC,
        )
        .map_err(CrossConeLayoutArtifactWriteError::Compatibility)?;
        let mut link_identity_closure =
            LinkIdentityClosureSectionV1::from_verified_layout_code(&code);
        if let Some(dependencies) = dependencies {
            let removed = dependencies.select(
                &link_objects,
                &link_identity_closure,
                code.production().projection().odr_members(),
            );
            link_objects.retain(|object| !removed.contains(&object.content().id()));
            link_identity_closure.remove_members(&removed);
        }

        let metadata = metadata.assemble(
            identity,
            lir_foundation,
            &code,
            &link_identity_closure,
            &link_support,
            &compatibility,
            &direct_dependencies,
        )?;

        let foundation_fingerprints = metadata.fingerprints;
        let semantic_fingerprints = SemanticFingerprintRecord::from_digests(
            foundation_fingerprints.hir(),
            foundation_fingerprints.mir(),
            foundation_fingerprints.lir(),
            FingerprintAvailability::Available(code.fingerprint()),
            FingerprintAvailability::Available(
                code.production().projection().runtime_image_fingerprint(),
            ),
        );

        let mut members = Vec::with_capacity(3 + link_objects.len());
        members.push(metadata.hir);
        members.push(metadata.mir);
        members.push(metadata.lir);
        members.extend(
            crate::member::encode_object_members(identity, link_objects)
                .map_err(CrossConeLayoutArtifactWriteError::ObjectMembers)?,
        );
        let mut manifest_sections = Vec::new();
        if let Some(directory) = &test_directory {
            manifest_sections.push(
                directory
                    .section()
                    .map_err(CrossConeLayoutArtifactWriteError::TestDirectory)?,
            );
        }
        let manifest = BootstrapManifest::new(
            producer,
            compatibility,
            cone,
            direct_dependencies,
            &members,
            semantic_fingerprints,
            manifest_sections,
        )
        .map_err(CrossConeLayoutArtifactWriteError::Manifest)?;
        let summary = crate::CrossConeArtifactSummary::from_layout(
            &manifest,
            target_selection,
            test_directory,
        );
        let archive = CanonicalSlibArchive::write_bootstrap(&manifest, members)
            .map_err(CrossConeLayoutArtifactWriteError::Archive)?;
        Ok(Self { archive, summary })
    }

    pub const fn artifact_fingerprint(&self) -> ArtifactFingerprint {
        self.summary.artifact_fingerprint()
    }

    pub const fn target_selection(&self) -> ValidatedLirTargetSelection {
        self.summary.target_selection()
    }

    pub const fn summary(&self) -> &crate::CrossConeArtifactSummary {
        &self.summary
    }

    pub fn publish(
        &self,
        destination: &std::path::Path,
    ) -> Result<crate::PublishedCrossConeArtifact, crate::CrossConeArtifactPublishError> {
        crate::PublishedCrossConeArtifact::write(&self.archive, &self.summary, destination)
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.archive.as_bytes()
    }
}

#[cfg(test)]
mod tests;
