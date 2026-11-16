use super::{DiagnosticOriginV1, HostPathCarrier, StructuredDiagnosticV1};

impl StructuredDiagnosticV1 {
    /// Replaces transport locators with retained artifact locators, preserving
    /// every semantic location and independently sourced note.
    pub fn remap_artifact_paths(
        &mut self,
        mut remap: impl FnMut(&HostPathCarrier) -> Option<HostPathCarrier>,
    ) {
        fn remap_origin(
            origin: &mut DiagnosticOriginV1,
            remap: &mut impl FnMut(&HostPathCarrier) -> Option<HostPathCarrier>,
        ) {
            if let DiagnosticOriginV1::ArtifactPath { path, .. } = origin
                && let Some(original) = remap(path)
            {
                *path = original;
            }
        }
        fn remap_notes(
            notes: &mut [super::DiagnosticNoteV1],
            remap: &mut impl FnMut(&HostPathCarrier) -> Option<HostPathCarrier>,
        ) {
            for note in notes {
                remap_origin(&mut note.origin, remap);
                remap_notes(&mut note.notes, remap);
            }
        }
        remap_origin(&mut self.origin, &mut remap);
        remap_notes(&mut self.notes, &mut remap);
    }
}

#[cfg(test)]
mod tests;
