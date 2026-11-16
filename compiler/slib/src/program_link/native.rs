use super::*;
use crate::{NativeLinkObject, SlibMemberRole, ValidatedGraphArtifact};

pub(super) fn read(
    graph: &mut ValidatedGraphArtifact<'_>,
) -> Result<Vec<NativeLinkObject>, ProgramLinkReadError> {
    let records: Vec<_> = graph
        .envelope
        .manifest()
        .members()
        .iter()
        .filter_map(|record| match record.role() {
            SlibMemberRole::NativeObject { source } => {
                Some((record.id(), source.clone(), record.sha256()))
            }
            _ => None,
        })
        .collect();
    records
        .into_iter()
        .map(|(member, source, digest)| {
            let bytes = graph
                .envelope
                .member(member)
                .map_err(|cause| error(crate::SlibDiagnostic::code(&cause), cause))?
                .ok_or_else(|| {
                    error(
                        scoop_source::DiagnosticCode::E2403,
                        format!("missing native object {member}"),
                    )
                })?;
            Ok(NativeLinkObject {
                member,
                source,
                bytes: bytes.into(),
                digest,
            })
        })
        .collect()
}
