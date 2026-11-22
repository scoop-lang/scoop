use scoop_identity::ConeIdentity;
use scoop_wire::{WireEncode, encode};

use super::StrongArtifactSectionV1;
use crate::{
    MemberPurposeSet, MemberStableKey, MetadataEnvelope, MetadataEnvelopeError, MetadataLocation,
    MetadataSection, MetadataSectionError, SlibMember, SlibMemberRecordError, SlibMemberRole,
};

pub(super) struct LayerAssembly {
    metadata: MetadataEnvelope,
}

impl LayerAssembly {
    pub(super) fn new(
        location: MetadataLocation,
        sections: Vec<MetadataSection>,
    ) -> Result<Self, StrongArtifactAssemblyError> {
        let metadata = MetadataEnvelope::new(location, sections)
            .map_err(|source| StrongArtifactAssemblyError::MetadataEnvelope { location, source })?;
        Ok(Self { metadata })
    }

    pub(super) fn sections(&self) -> &[MetadataSection] {
        self.metadata.sections()
    }

    pub(super) fn into_member(
        self,
        cone: ConeIdentity,
    ) -> Result<SlibMember, StrongArtifactAssemblyError> {
        let location = self.metadata.location();
        let (stable_key, role) = match location {
            MetadataLocation::Hir => (MemberStableKey::HirMetadata, SlibMemberRole::HirMetadata),
            MetadataLocation::Mir => (MemberStableKey::MirMetadata, SlibMemberRole::MirMetadata),
            MetadataLocation::Lir => (MemberStableKey::LirMetadata, SlibMemberRole::LirMetadata),
        };
        let envelope = encode(&self.metadata).map_err(|source| {
            StrongArtifactAssemblyError::MetadataEnvelopeEncoding { location, source }
        })?;
        drop(self);
        SlibMember::new(cone, stable_key, role, envelope)
            .map_err(|source| StrongArtifactAssemblyError::MetadataMember { location, source })
    }
}

pub(super) fn build_section(
    section: StrongArtifactSectionV1,
    location: MetadataLocation,
    capability: scoop_identity::CapabilityId,
    required_for: MemberPurposeSet,
    value: &impl WireEncode,
) -> Result<MetadataSection, StrongArtifactAssemblyError> {
    let payload = encode(value)
        .map_err(|source| StrongArtifactAssemblyError::Encoding { section, source })?;
    MetadataSection::new(location, capability, required_for, payload)
        .map_err(|source| StrongArtifactAssemblyError::MetadataSection { section, source })
}

#[derive(Debug)]
pub(super) enum StrongArtifactAssemblyError {
    Encoding {
        section: StrongArtifactSectionV1,
        source: scoop_wire::cbor::EncodeError,
    },
    MetadataSection {
        section: StrongArtifactSectionV1,
        source: MetadataSectionError,
    },
    MetadataEnvelope {
        location: MetadataLocation,
        source: MetadataEnvelopeError,
    },
    MetadataEnvelopeEncoding {
        location: MetadataLocation,
        source: scoop_wire::cbor::EncodeError,
    },
    MetadataMember {
        location: MetadataLocation,
        source: SlibMemberRecordError,
    },
}
