use super::*;

#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct NativeArchiveMemberId {
    pub input: NativeInputId,
    pub ordinal: u64,
    pub header_offset: u64,
    pub payload_offset: u64,
    pub payload_length: u64,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum NativeObjectId {
    Direct(NativeInputId),
    ArchiveMember(NativeArchiveMemberId),
}

impl NativeObjectId {
    pub fn input(self) -> NativeInputId {
        match self {
            Self::Direct(id) => id,
            Self::ArchiveMember(member) => member.input,
        }
    }
}
impl std::fmt::Display for NativeObjectId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Direct(id) => id.fmt(f),
            Self::ArchiveMember(member) => write!(
                f,
                "{}-member-{}-{}",
                member.input, member.ordinal, member.header_offset
            ),
        }
    }
}
impl WireEncode for NativeObjectId {
    fn encode(&self, e: &mut Encoder) -> Result<(), scoop_wire::cbor::EncodeError> {
        match self {
            Self::Direct(id) => {
                e.array(2)?;
                e.unsigned(1)?;
                id.encode(e)
            }
            Self::ArchiveMember(member) => {
                e.array(6)?;
                e.unsigned(2)?;
                member.input.encode(e)?;
                e.unsigned(member.ordinal)?;
                e.unsigned(member.header_offset)?;
                e.unsigned(member.payload_offset)?;
                e.unsigned(member.payload_length)
            }
        }
    }
}

impl NativeFile {
    pub fn candidate(&self, symbol: &str) -> Option<(NativeObjectId, &NativeObjectInfo)> {
        match &self.content {
            NativeContent::System | NativeContent::Dynamic(_) | NativeContent::ElfDynamic(_) => {
                None
            }
            NativeContent::Object(index) => index
                .definitions
                .contains_key(symbol)
                .then_some((NativeObjectId::Direct(self.id), index)),
            NativeContent::Archive(members) => members
                .iter()
                .filter(|member| !member.has_imports())
                .filter_map(|member| member.object_index().map(|index| (member, index)))
                .find(|(_, index)| index.definitions.contains_key(symbol))
                .map(|(member, index)| (NativeObjectId::ArchiveMember(member.id), index)),
        }
    }

    pub fn objects(&self) -> Vec<(NativeObjectId, &NativeObjectInfo, Range<usize>, Digest256)> {
        match &self.content {
            NativeContent::System | NativeContent::Dynamic(_) | NativeContent::ElfDynamic(_) => {
                Vec::new()
            }
            NativeContent::Object(index) => {
                vec![(
                    NativeObjectId::Direct(self.id),
                    index,
                    self.slice.clone(),
                    self.object_digest,
                )]
            }
            NativeContent::Archive(members) => members
                .iter()
                .filter_map(|member| member.object_index().map(|index| (member, index)))
                .map(|(member, index)| {
                    (
                        NativeObjectId::ArchiveMember(member.id),
                        index,
                        member.range.clone(),
                        member.digest,
                    )
                })
                .collect(),
        }
    }
}

impl NativeInputs {
    pub fn object(
        &self,
        id: NativeObjectId,
    ) -> Result<(&NativeObjectInfo, Range<usize>), LinkError> {
        self.files
            .get(&id.input())
            .and_then(|file| match (&file.content, id) {
                (NativeContent::Object(index), NativeObjectId::Direct(_)) => {
                    Some((index, file.slice.clone()))
                }
                (NativeContent::Archive(members), NativeObjectId::ArchiveMember(id)) => {
                    // The archive reader retains every member in ordinal order,
                    // including import members that do not have an object index.
                    let member = members.get(usize::try_from(id.ordinal).ok()?)?;
                    if member.id != id {
                        return None;
                    }
                    member
                        .object_index()
                        .map(|index| (index, member.range.clone()))
                }
                _ => None,
            })
            .ok_or_else(|| {
                error(
                    scoop_source::DiagnosticCode::ICE2201,
                    format!("missing native object {id}"),
                )
            })
    }
}
