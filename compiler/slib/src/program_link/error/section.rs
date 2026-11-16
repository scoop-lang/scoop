use crate::{ArtifactProfileInventoryError, SingleConeLinkSectionDecodeError, SlibDiagnostic};

use super::{ProgramLinkReadError, error};

pub(in crate::program_link) fn section(
    source: SingleConeLinkSectionDecodeError,
) -> ProgramLinkReadError {
    use SingleConeLinkSectionDecodeError::*;
    use scoop_source::DiagnosticCode as Code;
    let code = match &source {
        MemberRead(cause) => cause.code(),
        OuterEnvelope { source, .. } => source.code(),
        WrongProfile { .. } => Code::E2404,
        MissingMetadataMember { .. }
        | MissingMetadataMemberPayload { .. }
        | MissingSection { .. } => Code::E2403,
        InnerSection { source, .. } | Resource(source) => source.code(),
        Inventory(cause) => match cause {
            ArtifactProfileInventoryError::ConflictingCapabilityVersion { .. } => Code::E2402,
            ArtifactProfileInventoryError::UnsupportedRequiredCapability { .. } => Code::E2411,
            ArtifactProfileInventoryError::MissingRequiredCapability { .. } => Code::E2403,
        },
    };
    let mut result = error(code, &source);
    result.semantic_path = path(&source);
    result
}

pub(super) fn path(source: &SingleConeLinkSectionDecodeError) -> String {
    use SingleConeLinkSectionDecodeError::*;
    match source {
        MemberRead(error) => error.diagnostic().semantic_path(),
        WrongProfile { .. } => "manifest:compatibility.artifact_profile".to_owned(),
        MissingMetadataMember { location } => format!("{location}:$"),
        MissingMetadataMemberPayload { member, .. } => format!("member/{member}:$"),
        OuterEnvelope { location, source } => format!("{location}:{}", source.diagnostic().path()),
        MissingSection {
            location,
            capability,
        } => format!("{}:$", name(*location, capability)),
        InnerSection {
            location,
            capability,
            source,
        } => format!("{}:{}", name(*location, capability), source.path()),
        Resource(source) => source.diagnostic().semantic_path(),
        Inventory(source) => match source {
            ArtifactProfileInventoryError::ConflictingCapabilityVersion {
                location, index, ..
            }
            | ArtifactProfileInventoryError::UnsupportedRequiredCapability {
                location,
                index,
                ..
            } => format!("{location}:sections[{index}]"),
            ArtifactProfileInventoryError::MissingRequiredCapability { location, .. } => {
                format!("{location}:sections")
            }
        },
    }
}

fn name(
    location: Option<crate::MetadataLocation>,
    capability: &scoop_identity::CapabilityId,
) -> String {
    let location = location.map_or_else(|| "manifest".to_owned(), |it| it.to_string());
    format!(
        "{location}/{}/{}/{}",
        capability.namespace(),
        capability.name(),
        capability.major_version()
    )
}
