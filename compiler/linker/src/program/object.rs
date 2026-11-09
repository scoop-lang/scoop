use super::ObjectOrigin;
use crate::{LinkError, error};
use scoop_wire::Digest256;
use std::borrow::Cow;
use std::ops::Range;
use std::path::{Path, PathBuf};
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

    pub fn path(&self, cache: &Path, darwin: bool) -> Result<PathBuf, LinkError> {
        if !darwin && let ObjectBytes::File { path } = &self.bytes {
            return Ok(path.clone());
        }
        let digest = if darwin {
            scoop_wire::domain_separated_cbor_hash("scoop-darwin-link-object-v1", &self.digest)
                .map_err(error)?
        } else {
            self.digest
        };
        let path = scoop_toolchain::object_path(cache, digest).map_err(error)?;
        if path.try_exists().map_err(error)? {
            return Ok(path);
        }
        let bytes: Cow<'_, [u8]> = match &self.bytes {
            ObjectBytes::Artifact { artifact, member } => {
                Cow::Borrowed(artifact.object_bytes(*member).map_err(error)?)
            }
            ObjectBytes::Native { file, range } => Cow::Borrowed(&file[range.clone()]),
            ObjectBytes::File { path } => Cow::Owned(std::fs::read(path).map_err(error)?),
        };
        let bytes = if darwin {
            Cow::Owned(scoop_slib::remove_empty_macho_code_labels(&bytes).map_err(error)?)
        } else {
            bytes
        };
        scoop_toolchain::store_object(cache, digest, &bytes).map_err(error)
    }
}
