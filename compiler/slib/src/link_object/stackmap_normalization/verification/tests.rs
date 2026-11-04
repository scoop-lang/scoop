use super::*;

pub(crate) mod support;
use support::{Corruption, Fixture};

#[test]
fn reads_actual_stackmap_records_and_return_addresses() {
    let fixture = Fixture::new(Corruption::None);
    let objects = [ScoopLirObjectCandidateV1::new(
        fixture.member,
        &fixture.object_bytes,
    )];

    let verified =
        read_scoop_stackmaps_v1(fixture.builtins, fixture.semantic_plan, &objects).unwrap();

    assert_eq!(
        verified.producer(),
        scoop_identity::ConeIdentity::SINGLE_FILE
    );
    assert_eq!(verified.records().len(), 2);
    assert!(
        verified
            .records()
            .windows(2)
            .all(|pair| pair[0].normalized().canonical().site()
                < pair[1].normalized().canonical().site())
    );
    assert_eq!(
        verified
            .records()
            .iter()
            .map(|record| record.normalized().canonical().site())
            .collect::<Vec<_>>(),
        verified
            .semantic_plan()
            .sites()
            .iter()
            .map(|site| site.site())
            .collect::<Vec<_>>()
    );
    assert!(verified.records().iter().all(|record| {
        record.member() == fixture.member
            && record.normalized().canonical().root_pair_count() == 0
            && record.normalized().canonical().locations().len() == 3
    }));
    let mut return_pcs = verified
        .records()
        .iter()
        .map(ScoopStackmapRecordV1::object_return_pc)
        .collect::<Vec<_>>();
    return_pcs.sort_unstable();
    assert_eq!(return_pcs, [12, 16]);
}

#[test]
fn reports_an_unknown_safepoint_reference() {
    let fixture = Fixture::new(Corruption::UnknownSafepoint);
    let member = fixture.member;
    let objects = [ScoopLirObjectCandidateV1::new(
        member,
        &fixture.object_bytes,
    )];

    assert!(matches!(
        read_scoop_stackmaps_v1(fixture.builtins, fixture.semantic_plan, &objects),
        Err(ScoopStackmapReadError::UnexpectedSafepointId {
            member: actual,
            ..
        }) if actual == member
    ));
}

#[test]
fn compact_macho_map_preserves_code_symbols_and_other_relocations() {
    use object::{Object, ObjectSection, ObjectSymbol};

    let fixture = Fixture::new(Corruption::None);
    let objects = [ScoopLirObjectCandidateV1::new(
        fixture.member,
        &fixture.object_bytes,
    )];
    let records =
        read_scoop_stackmaps_v1(fixture.builtins, fixture.semantic_plan, &objects).unwrap();
    let compact = crate::compact_callable_gc_object_v1(
        scoop_target_data::DARWIN_AARCH64,
        &fixture.object_bytes,
        &records.records().iter().collect::<Vec<_>>(),
    )
    .unwrap();
    let before = object::File::parse(fixture.object_bytes.as_slice()).unwrap();
    let after = object::File::parse(compact.as_slice()).unwrap();
    let raw = before.section_by_name("__llvm_stackmaps").unwrap();
    let map = after.section_by_name("__scoop_gc").unwrap();
    assert_eq!(map.index(), raw.index());
    assert_eq!(map.segment_name().unwrap(), Some("__DATA_CONST"));
    assert_eq!(map.size(), 72);
    assert!(map.size() < raw.size());
    assert_eq!(map.relocations().count(), 0);
    assert!(after.section_by_name("__llvm_stackmaps").is_none());
    for section in before
        .sections()
        .filter(|section| section.index() != raw.index())
    {
        let other = after.section_by_index(section.index()).unwrap();
        assert_eq!(section.data().unwrap(), other.data().unwrap());
        assert_eq!(section.address(), other.address());
        let relocations = |section: &object::Section<'_, '_>| {
            section
                .relocations()
                .map(|(offset, relocation)| {
                    (
                        offset,
                        relocation.target(),
                        relocation.addend(),
                        relocation.flags(),
                    )
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(relocations(&section), relocations(&other));
    }
    for symbol in before.symbols() {
        let other = after.symbol_by_index(symbol.index()).unwrap();
        assert_eq!(symbol.name_bytes().unwrap(), other.name_bytes().unwrap());
        let expected = if symbol.section_index() == Some(raw.index())
            && symbol.address() == raw.address() + raw.size()
        {
            map.address() + map.size()
        } else {
            symbol.address()
        };
        assert_eq!(other.address(), expected);
    }
}
