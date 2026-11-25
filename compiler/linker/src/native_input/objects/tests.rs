use super::*;
use crate::{NativeSymbolBinding, NativeSymbolDefinition, NativeSymbolKind};

#[test]
fn native_lookup_uses_complete_member_ids_and_preserves_import_ordinals() {
    let input = NativeInputId(sha256(b"archive"));
    let definition = NativeSymbolDefinition {
        kind: NativeSymbolKind::Function,
        read_only: true,
        binding: NativeSymbolBinding::Strong,
    };
    let info = |symbol: &str| NativeObjectInfo {
        definitions: BTreeMap::from([(symbol.to_owned(), definition)]),
        requirements: Default::default(),
    };
    let indexes = [
        archive::MemberIndex::Object {
            info: info("first"),
            imports: false,
        },
        archive::MemberIndex::Import(coff_import::CoffImport {
            dll: "provider.dll".into(),
            symbol: "imported".into(),
            definition,
        }),
        archive::MemberIndex::Object {
            info: info("last"),
            imports: false,
        },
    ];
    let members: Vec<_> = indexes
        .into_iter()
        .enumerate()
        .map(|(ordinal, index)| {
            let start = ordinal * 64 + 16;
            archive::Member {
                id: NativeArchiveMemberId {
                    input,
                    ordinal: ordinal as u64,
                    header_offset: (ordinal * 64) as u64,
                    payload_offset: start as u64,
                    payload_length: 4,
                },
                name: ordinal.to_string(),
                range: start..start + 4,
                digest: sha256(&[ordinal as u8; 4]),
                index,
            }
        })
        .collect();
    let ids: Vec<_> = members.iter().map(|member| member.id).collect();
    let mut inputs = NativeInputs::default();
    inputs.files.insert(
        input,
        NativeFile {
            id: input,
            object_digest: sha256(b"archive"),
            object_path: None,
            bytes: Arc::from([]),
            slice: 0..0,
            locator: "archive.a".into(),
            content: NativeContent::Archive(members),
        },
    );
    for (ordinal, symbol) in [(0, "first"), (2, "last")] {
        let (index, range) = inputs
            .object(NativeObjectId::ArchiveMember(ids[ordinal]))
            .unwrap();
        assert!(index.definitions.contains_key(symbol));
        assert_eq!(range, ordinal * 64 + 16..ordinal * 64 + 20);
    }
    assert!(
        inputs
            .object(NativeObjectId::ArchiveMember(ids[1]))
            .is_err()
    );
    assert!(inputs.object(NativeObjectId::Direct(input)).is_err());
    for invalid in [
        NativeArchiveMemberId {
            ordinal: u64::MAX,
            ..ids[0]
        },
        NativeArchiveMemberId {
            header_offset: 1,
            ..ids[0]
        },
        NativeArchiveMemberId {
            payload_offset: 1,
            ..ids[0]
        },
        NativeArchiveMemberId {
            payload_length: 1,
            ..ids[0]
        },
    ] {
        assert!(
            inputs
                .object(NativeObjectId::ArchiveMember(invalid))
                .is_err()
        );
    }

    let direct = NativeInputId(sha256(b"direct"));
    inputs.files.insert(
        direct,
        NativeFile {
            id: direct,
            object_digest: sha256(b"direct"),
            object_path: None,
            bytes: Arc::from([0; 8]),
            slice: 2..6,
            locator: "direct.o".into(),
            content: NativeContent::Object(info("direct")),
        },
    );
    let (index, range) = inputs.object(NativeObjectId::Direct(direct)).unwrap();
    assert!(index.definitions.contains_key("direct"));
    assert_eq!(range, 2..6);
    assert!(
        inputs
            .object(NativeObjectId::ArchiveMember(NativeArchiveMemberId {
                input: direct,
                ..ids[0]
            }))
            .is_err()
    );
}
