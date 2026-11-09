use super::ObjectOrigin;
use crate::{LinkError, error};
use scoop_wire::Digest256;
use std::borrow::Cow;
use std::ops::Range;
use std::path::PathBuf;
use std::sync::Arc;

pub(crate) enum ObjectBytes<'a> {
    Artifact {
        artifact: &'a scoop_slib::ProgramLinkArtifact,
        member: scoop_slib::SlibMemberId,
    },
    Native {
        file: Arc<[u8]>,
        range: Range<usize>,
    },
    File {
        path: PathBuf,
    },
}

pub(crate) struct InputObject<'a> {
    pub origin: ObjectOrigin,
    pub bytes: ObjectBytes<'a>,
    pub digest: Digest256,
}
impl InputObject<'_> {
    pub fn digest(&self) -> Digest256 {
        self.digest
    }

    pub fn read_bytes(&self) -> Result<Cow<'_, [u8]>, LinkError> {
        Ok(match &self.bytes {
            ObjectBytes::Artifact { artifact, member } => {
                Cow::Borrowed(artifact.object_bytes(*member).map_err(error)?)
            }
            ObjectBytes::Native { file, range } => Cow::Borrowed(&file[range.clone()]),
            ObjectBytes::File { path } => Cow::Owned(std::fs::read(path).map_err(error)?),
        })
    }
}
