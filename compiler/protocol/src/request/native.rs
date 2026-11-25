use super::*;

/// Locations used by the child to produce or reuse its own native objects.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeBuildPathsV1 {
    pub sysroot: HostPathCarrier,
    pub cache: HostPathCarrier,
}

pub(super) fn encode(
    paths: &Option<NativeBuildPathsV1>,
    e: &mut Encoder,
) -> Result<(), scoop_wire::cbor::EncodeError> {
    e.array(u64::from(paths.is_some()))?;
    if let Some(paths) = paths {
        e.array(2)?;
        paths.sysroot.encode(e)?;
        paths.cache.encode(e)?;
    }
    Ok(())
}

pub(super) fn decode(d: &mut Decoder<'_>) -> Result<Option<NativeBuildPathsV1>, WireError> {
    match d.array()? {
        0 => Ok(None),
        1 => {
            if d.array()? != 2 {
                return Err(crate::compiler_inputs::invalid(d));
            }
            Ok(Some(NativeBuildPathsV1 {
                sysroot: crate::compiler_inputs::decode_path(d)?,
                cache: crate::compiler_inputs::decode_path(d)?,
            }))
        }
        _ => Err(crate::compiler_inputs::invalid(d)),
    }
}
