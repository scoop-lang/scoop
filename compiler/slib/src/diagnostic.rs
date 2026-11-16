use std::cmp::Ordering;

use scoop_identity::{CapabilityId, ConeIdentity};
use scoop_wire::{WireError, WireErrorKind, WirePath};

use crate::{
    ArchiveMemberOrdinal, ArchiveReadError, BootstrapManifestError,
    BootstrapManifestValidationError, CompatibilitySchemaKind, CompatibilityValidationError,
    GraphValidationError, MetadataLocation, MetadataReadError, MetadataSectionError,
    MetadataSectionValidationError, ProducerRecordError, SchemaKind,
    SemanticFingerprintValidationError, SlibMemberId, SlibReadError,
};

pub use scoop_source::DiagnosticCode;

/// Stable origin of the primary failing value when one is already trusted.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum SlibPrimaryOrigin {
    Container,
    Manifest,
    Metadata(MetadataLocation),
    Member(SlibMemberId),
    Cone(ConeIdentity),
    Capability {
        location: Option<MetadataLocation>,
        capability: CapabilityId,
    },
    Identity {
        kind: &'static str,
        id: [u8; 32],
    },
}

/// Renderer-independent projection of one typed `.slib` failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SlibDiagnosticRecord {
    code: DiagnosticCode,
    path: WirePath,
    byte_offset: Option<u64>,
    primary_origin: Option<SlibPrimaryOrigin>,
}

impl Ord for SlibDiagnosticRecord {
    fn cmp(&self, other: &Self) -> Ordering {
        self.primary_origin
            .cmp(&other.primary_origin)
            .then_with(|| self.path.cmp(&other.path))
            .then_with(|| self.code.cmp(&other.code))
            .then_with(|| self.byte_offset.cmp(&other.byte_offset))
    }
}

impl PartialOrd for SlibDiagnosticRecord {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl SlibDiagnosticRecord {
    pub const fn code(&self) -> DiagnosticCode {
        self.code
    }

    pub const fn path(&self) -> &WirePath {
        &self.path
    }

    pub const fn byte_offset(&self) -> Option<u64> {
        self.byte_offset
    }

    pub const fn primary_origin(&self) -> Option<&SlibPrimaryOrigin> {
        self.primary_origin.as_ref()
    }

    fn new(code: DiagnosticCode, path: WirePath) -> Self {
        Self {
            code,
            path,
            byte_offset: None,
            primary_origin: None,
        }
    }

    fn with_origin(mut self, origin: SlibPrimaryOrigin) -> Self {
        self.primary_origin = Some(origin);
        self
    }
}

/// Produces stable machine-readable diagnostics without parsing `Display`.
pub trait SlibDiagnostic {
    fn code(&self) -> DiagnosticCode {
        self.diagnostic().code()
    }
    fn diagnostic(&self) -> SlibDiagnosticRecord;
}

mod compile;
mod location;
mod record_helpers;
use record_helpers::{
    cone_record_diagnostic, dependency_record_diagnostic, member_record_diagnostic,
};

pub(super) fn root_diagnostic(
    code: DiagnosticCode,
    origin: SlibPrimaryOrigin,
) -> SlibDiagnosticRecord {
    SlibDiagnosticRecord::new(code, WirePath::root()).with_origin(origin)
}

#[cfg(test)]
mod tests;

impl SlibDiagnostic for WireError {
    fn diagnostic(&self) -> SlibDiagnosticRecord {
        let code = match self.kind() {
            WireErrorKind::UnexpectedEnd => DiagnosticCode::E2401,
            WireErrorKind::WrongType { .. } => DiagnosticCode::E2401,
            WireErrorKind::IndefiniteLength { .. } => DiagnosticCode::E2401,
            WireErrorKind::NonCanonicalCbor => DiagnosticCode::E2401,
            WireErrorKind::TrailingData => DiagnosticCode::E2401,
            WireErrorKind::DuplicateField { .. }
            | WireErrorKind::MissingField { .. }
            | WireErrorKind::ExtraField { .. }
            | WireErrorKind::UnexpectedField { .. } => DiagnosticCode::E2401,
            WireErrorKind::UnknownTag { .. } => DiagnosticCode::E2401,
            WireErrorKind::InvalidLength { .. } => DiagnosticCode::E2401,
            WireErrorKind::IntegerOutOfRange | WireErrorKind::InvalidIndex { .. } => {
                DiagnosticCode::E2401
            }

            WireErrorKind::Allocation => DiagnosticCode::E2205,
        };
        SlibDiagnosticRecord {
            code,
            path: self.path().clone(),
            byte_offset: self.byte_offset(),
            primary_origin: None,
        }
    }
}

impl SlibDiagnostic for ArchiveReadError {
    fn diagnostic(&self) -> SlibDiagnosticRecord {
        match self {
            Self::BadMagic => root_diagnostic(DiagnosticCode::E2401, SlibPrimaryOrigin::Container),
            Self::TruncatedHeader { member } | Self::TruncatedPayload { member } => {
                archive_member_diagnostic(DiagnosticCode::E2401, *member)
            }
            Self::Compression { member, .. } => {
                archive_member_diagnostic(DiagnosticCode::E2410, *member)
            }
            Self::LengthOverflow => {
                root_diagnostic(DiagnosticCode::E2401, SlibPrimaryOrigin::Container)
            }
            Self::NonIncreasingDirectory { second_index, .. } => SlibDiagnosticRecord::new(
                DiagnosticCode::E2401,
                WirePath::root()
                    .field(8)
                    .index(u64::try_from(*second_index).unwrap_or(u64::MAX)),
            )
            .with_origin(SlibPrimaryOrigin::Manifest),
        }
    }
}

impl SlibDiagnostic for SlibReadError {
    fn diagnostic(&self) -> SlibDiagnosticRecord {
        match self {
            Self::Io(_) => SlibDiagnosticRecord::new(DiagnosticCode::E2201, WirePath::root())
                .with_origin(SlibPrimaryOrigin::Container),
            Self::TestDirectory(_) => {
                SlibDiagnosticRecord::new(DiagnosticCode::E2405, WirePath::root())
                    .with_origin(SlibPrimaryOrigin::Manifest)
            }
            Self::Container(error) | Self::Directory(error) => error.diagnostic(),
            Self::CanonicalWire(error) => {
                error.diagnostic().with_origin(SlibPrimaryOrigin::Manifest)
            }
            Self::Manifest(error) => error.diagnostic(),
        }
    }
}

impl SlibDiagnostic for BootstrapManifestValidationError {
    fn diagnostic(&self) -> SlibDiagnosticRecord {
        let manifest = SlibPrimaryOrigin::Manifest;
        match self {
            Self::BadMagic => {
                SlibDiagnosticRecord::new(DiagnosticCode::E2401, WirePath::root().field(1))
                    .with_origin(manifest)
            }
            Self::UnsupportedSchema { kind, .. } => SlibDiagnosticRecord::new(
                DiagnosticCode::E2402,
                WirePath::root().field(match kind {
                    SchemaKind::Manifest => 2,
                    SchemaKind::Container => 3,
                }),
            )
            .with_origin(manifest),
            Self::Producer(ProducerRecordError::TooLong { .. }) => {
                SlibDiagnosticRecord::new(DiagnosticCode::E2401, WirePath::root().field(4).field(1))
                    .with_origin(manifest)
            }
            Self::Compatibility(error) => compatibility_diagnostic(error),
            Self::Cone(error) => cone_record_diagnostic(error),
            Self::Dependency { index, error } => dependency_record_diagnostic(*index, error),
            Self::NonIncreasingDependency { second_index, .. } => SlibDiagnosticRecord::new(
                DiagnosticCode::E2405,
                WirePath::root()
                    .field(7)
                    .index(u64::try_from(*second_index).unwrap_or(u64::MAX)),
            )
            .with_origin(manifest),
            Self::Member { index, error } => member_record_diagnostic(*index, error),
            Self::NonIncreasingMember { second_index, .. } => SlibDiagnosticRecord::new(
                DiagnosticCode::E2401,
                WirePath::root()
                    .field(8)
                    .index(u64::try_from(*second_index).unwrap_or(u64::MAX)),
            )
            .with_origin(manifest),
            Self::SemanticFingerprints(error) => SlibDiagnosticRecord::new(
                DiagnosticCode::E2108,
                WirePath::root().field(9).field(match error {
                    SemanticFingerprintValidationError::CodeMustBeAvailable => 4,
                    SemanticFingerprintValidationError::RuntimeImageMustBeAvailable => 5,
                }),
            )
            .with_origin(manifest),
            Self::Section { index, .. }
            | Self::NonIncreasingSection {
                second_index: index,
                ..
            } => SlibDiagnosticRecord::new(
                DiagnosticCode::E2401,
                WirePath::root()
                    .field(10)
                    .index(u64::try_from(*index).unwrap_or(u64::MAX)),
            )
            .with_origin(manifest),
            Self::Manifest(error) => manifest_build_diagnostic(error),
            Self::Wire(error) => error.diagnostic().with_origin(manifest),
        }
    }
}

impl SlibDiagnostic for MetadataReadError {
    fn diagnostic(&self) -> SlibDiagnosticRecord {
        match self {
            Self::CanonicalWire(error) => error.diagnostic(),
            Self::InvalidIdTable => {
                SlibDiagnosticRecord::new(DiagnosticCode::E2401, WirePath::root().field(4))
            }
            Self::BadMagic { expected } => {
                SlibDiagnosticRecord::new(DiagnosticCode::E2401, WirePath::root().field(1))
                    .with_origin(SlibPrimaryOrigin::Metadata(*expected))
            }
            Self::UnsupportedSchema { .. } => {
                SlibDiagnosticRecord::new(DiagnosticCode::E2402, WirePath::root().field(2))
            }
            Self::Section { index, error } => {
                metadata_section_diagnostic(error).with_metadata_index(*index)
            }
            Self::NonIncreasingSection { second_index, .. } => SlibDiagnosticRecord::new(
                DiagnosticCode::E2401,
                WirePath::root()
                    .field(3)
                    .index(u64::try_from(*second_index).unwrap_or(u64::MAX)),
            ),
            Self::Allocation => {
                SlibDiagnosticRecord::new(DiagnosticCode::E2205, WirePath::root().field(3))
            }
        }
    }
}

impl SlibDiagnosticRecord {
    fn with_metadata_index(mut self, index: usize) -> Self {
        self.path = WirePath::root()
            .field(3)
            .index(u64::try_from(index).unwrap_or(u64::MAX));
        self
    }
}

impl SlibDiagnostic for GraphValidationError {
    fn diagnostic(&self) -> SlibDiagnosticRecord {
        match self {
            Self::SelfDependency { cone } => {
                SlibDiagnosticRecord::new(DiagnosticCode::E2405, WirePath::root().field(7))
                    .with_origin(SlibPrimaryOrigin::Cone(*cone))
            }
            Self::SingleFileDependency => {
                SlibDiagnosticRecord::new(DiagnosticCode::E2405, WirePath::root().field(7))
            }
        }
    }
}

fn compatibility_diagnostic(error: &CompatibilityValidationError) -> SlibDiagnosticRecord {
    let (code, field) = match error {
        CompatibilityValidationError::UnsupportedSelection => (DiagnosticCode::E2404, 5),
        CompatibilityValidationError::UnsupportedSchema { kind, .. } => (
            DiagnosticCode::E2402,
            match kind {
                CompatibilitySchemaKind::Identity => 3,
                CompatibilitySchemaKind::Hir => 9,
                CompatibilitySchemaKind::Mir => 10,
                CompatibilitySchemaKind::Lir => 11,
            },
        ),
        CompatibilityValidationError::ManglingSchema { .. } => (DiagnosticCode::E2402, 4),
        CompatibilityValidationError::Capability(_) => (DiagnosticCode::E2401, 5),
        CompatibilityValidationError::TargetProfile(_)
        | CompatibilityValidationError::BackendProfile(_) => (DiagnosticCode::E2404, 5),
        CompatibilityValidationError::ProfileMismatch { kind, .. } => (
            DiagnosticCode::E2404,
            match kind {
                crate::CompatibilityProfileKind::Target => 5,
                crate::CompatibilityProfileKind::Backend => 7,
            },
        ),
        CompatibilityValidationError::ArtifactProfile(_) => (DiagnosticCode::E2404, 13),
        CompatibilityValidationError::FingerprintMismatch { kind, .. } => (
            DiagnosticCode::E2108,
            match kind {
                crate::CompatibilityFingerprintKind::LanguageAbi => 1,
                crate::CompatibilityFingerprintKind::RuntimeAbi => 2,
                crate::CompatibilityFingerprintKind::TargetProfile => 6,
                crate::CompatibilityFingerprintKind::BackendProfile => 8,
                crate::CompatibilityFingerprintKind::CompositeIdentityAbi => 12,
                crate::CompatibilityFingerprintKind::ArtifactProfile => 14,
            },
        ),
        CompatibilityValidationError::Hash(_) => (DiagnosticCode::E2108, 5),
        CompatibilityValidationError::Resource(error) => {
            return error.diagnostic().with_origin(SlibPrimaryOrigin::Manifest);
        }
    };
    SlibDiagnosticRecord::new(code, WirePath::root().field(5).field(field))
        .with_origin(SlibPrimaryOrigin::Manifest)
}

fn manifest_build_diagnostic(error: &BootstrapManifestError) -> SlibDiagnosticRecord {
    let (code, path, origin) = match error {
        BootstrapManifestError::DuplicateDependency { identity } => (
            DiagnosticCode::E2405,
            WirePath::root().field(7),
            Some(SlibPrimaryOrigin::Cone(*identity)),
        ),
        BootstrapManifestError::DuplicateMember { id } => (
            DiagnosticCode::E2401,
            WirePath::root().field(8).key("slib-member", *id.as_array()),
            Some(SlibPrimaryOrigin::Member(*id)),
        ),

        BootstrapManifestError::MissingMetadata { .. }
        | BootstrapManifestError::DuplicateMetadata { .. }
        | BootstrapManifestError::DuplicateObjectDictionary => (
            DiagnosticCode::E2401,
            WirePath::root().field(8),
            Some(SlibPrimaryOrigin::Manifest),
        ),
        BootstrapManifestError::DuplicateSection { capability } => (
            DiagnosticCode::E2401,
            WirePath::root().field(10),
            Some(SlibPrimaryOrigin::Capability {
                location: None,
                capability: capability.clone(),
            }),
        ),
        BootstrapManifestError::Hash(_) => (
            DiagnosticCode::E2108,
            WirePath::root().field(11),
            Some(SlibPrimaryOrigin::Manifest),
        ),
        BootstrapManifestError::Resource(error) => {
            return error.diagnostic().with_origin(SlibPrimaryOrigin::Manifest);
        }
    };
    let mut diagnostic = SlibDiagnosticRecord::new(code, path);
    diagnostic.primary_origin = origin;
    diagnostic
}

fn metadata_section_diagnostic(error: &MetadataSectionValidationError) -> SlibDiagnosticRecord {
    match error {
        MetadataSectionValidationError::Capability(_) => {
            SlibDiagnosticRecord::new(DiagnosticCode::E2401, WirePath::root())
        }
        MetadataSectionValidationError::Section(error) => match error {
            MetadataSectionError::InvalidPurpose { .. }
            | MetadataSectionError::KnownCapabilityWrongLocation { .. }
            | MetadataSectionError::KnownCapabilityWrongPurpose { .. } => {
                SlibDiagnosticRecord::new(DiagnosticCode::E2401, WirePath::root())
            }
        },
    }
}

fn archive_member_diagnostic(
    code: DiagnosticCode,
    member: ArchiveMemberOrdinal,
) -> SlibDiagnosticRecord {
    let (path, origin) = match member {
        ArchiveMemberOrdinal::Manifest => (WirePath::root(), SlibPrimaryOrigin::Manifest),
        ArchiveMemberOrdinal::Directory(index) => (
            WirePath::root().field(8).index(u64::from(index)),
            SlibPrimaryOrigin::Container,
        ),
    };
    SlibDiagnosticRecord::new(code, path).with_origin(origin)
}

impl crate::ArtifactManifestSummaryError {
    pub fn code(&self) -> DiagnosticCode {
        match self {
            Self::Envelope(e) => e.code(),
            Self::Io(_) => DiagnosticCode::E2201,
            Self::TestDirectory(_) => DiagnosticCode::E2406,
        }
    }
}

impl crate::CrossConeArtifactPublishError {
    pub fn code(&self) -> DiagnosticCode {
        match self {
            Self::MissingParent { .. } => DiagnosticCode::E2020,
            Self::Io { .. } => DiagnosticCode::E2201,
        }
    }
}

impl crate::CrossConeArtifactClosureValidationError {
    pub fn code(&self) -> DiagnosticCode {
        match self {
            Self::Layout(e) => e.code(),
            Self::Commit(e) => e.code(),
            Self::MissingCompletedCurrentArtifact => DiagnosticCode::ICE2401,
        }
    }
}

impl crate::CrossConeSemanticCommitError {
    pub fn code(&self) -> DiagnosticCode {
        match self {
            Self::StateCountMismatch { .. } => DiagnosticCode::ICE2401,
            Self::Allocation { .. } => DiagnosticCode::E2205,
            Self::IdentityPreparation { source, .. } => source.code(),
            Self::SemanticImport(e) => e.code(),
        }
    }
}

impl crate::CrossConeLayoutArtifactValidationError {
    pub fn code(&self) -> DiagnosticCode {
        match self {
            Self::Envelope { source, .. } => source.code(),
            Self::Graph { source, .. } => source.code(),
            Self::LinkSections { .. } => DiagnosticCode::E2409,
            Self::CompileSections { .. } => DiagnosticCode::E2406,
            Self::Semantic { source, .. } => source.code(),
            Self::Physical { .. } => DiagnosticCode::E2408,
            Self::Resource { source } => source.code(),
            Self::TestDirectory(_) => DiagnosticCode::E2406,
            Self::MissingCurrentArtifact => DiagnosticCode::ICE2401,
        }
    }
}

impl crate::CrossConeLayoutSemanticClosureError {
    pub fn code(&self) -> DiagnosticCode {
        match self {
            Self::Graph(e) => e.code(),
            Self::Identity(_) => DiagnosticCode::E2405,
            Self::Foundation(_)
            | Self::HirResolution(_)
            | Self::HirProduction(_)
            | Self::HirDeclarations(_) => DiagnosticCode::E2406,
            Self::SourceCallables(_) | Self::MirDependencies(_) => DiagnosticCode::E2407,
            Self::LirLayouts(_)
            | Self::LirCallableAbis(_)
            | Self::LirDispatch(_)
            | Self::LirDescriptors(_)
            | Self::LirShapeSupport(_)
            | Self::OrdinaryLirBridges(_)
            | Self::LirStrongProduction(_)
            | Self::LirDependencies(_) => DiagnosticCode::E2408,
        }
    }
}

impl crate::CrossConeClosureGraphError {
    pub fn code(&self) -> DiagnosticCode {
        match self {
            Self::NonCanonicalDirectProviders { .. } | Self::InvalidDependencyFirstOrder { .. } => {
                DiagnosticCode::E2401
            }
            Self::TargetMismatch { .. } => DiagnosticCode::E2404,
            Self::MultipleVersions { .. } => DiagnosticCode::E2104,
            Self::InvalidProviderKind { .. } => DiagnosticCode::E2103,
            Self::MissingDirectArtifact { .. } | Self::MissingDependencyArtifact { .. } => {
                DiagnosticCode::E2101
            }
            Self::StaleDependency { .. } => DiagnosticCode::E2108,
            Self::DuplicateArtifact { .. }
            | Self::CurrentArtifactPresent { .. }
            | Self::CurrentArtifactIdentityMismatch { .. }
            | Self::CurrentDirectSetMismatch { .. }
            | Self::UnreachableSupport { .. } => DiagnosticCode::E2405,
        }
    }
}
