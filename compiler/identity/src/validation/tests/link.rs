use super::*;

#[test]
fn shared_diamond_dependencies_visit_each_sources_records_once() {
    let core = validated_source_type_graph();
    let branch = |name: &str| {
        let cone = ConeCoordinate::new("tests", name, "1.0.0")
            .unwrap()
            .identity()
            .unwrap();
        let declaration =
            CborIdentityRecord::<PersistentTypeId, _>::from_key(SourceDeclarationKey::nominal(
                SourceDeclarationSite::new(
                    cone,
                    PackagePath::root(),
                    DefinitionOwnerChain::top_level(),
                    DeclarationScope::ConeWide,
                )
                .unwrap(),
                CanonicalIdentifier::new("Value").unwrap(),
                SourceNominalKind::Struct,
                0,
            ))
            .unwrap();
        let mut pending = PendingIdentityValidation::new();
        pending.register_authority(cone).unwrap();
        pending.register_external_graph_authorities(&core).unwrap();
        pending
            .register_canonical(IdentityLayer::Hir, declaration)
            .unwrap();
        pending.finish().unwrap()
    };
    let left = branch("left");
    let right = branch("right");
    let mut pending = PendingIdentityValidation::for_link();
    pending.register_external_graph_authorities(&left).unwrap();
    pending.register_external_graph_authorities(&right).unwrap();
    pending.register_external_graph_authorities(&core).unwrap();
    pending.register_external_graph_authorities(&left).unwrap();
    assert_eq!(
        pending.external_candidate_visits,
        [&core, &left, &right]
            .iter()
            .map(|graph| graph.local_nodes.len() + graph.declared_identity_count())
            .sum::<usize>()
    );
    let graph = pending.finish().unwrap();
    assert_eq!(graph.identity_count(), 6);
    assert_eq!(graph.declared_identity_count(), 0);
    assert_eq!(
        graph
            .closure_records::<PersistentTypeId, SourceDeclarationKey>(&WirePath::root())
            .unwrap()
            .len(),
        3
    );
    assert_eq!(
        left.records::<PersistentTypeId, SourceDeclarationKey>(
            IdentityLayer::Hir,
            &WirePath::root()
        )
        .unwrap()
        .len(),
        1
    );
}

#[test]
fn link_uses_saved_declarations_and_still_resolves_typed_references() {
    let mut bytes = encode(&source_type_record()).unwrap();
    // Link consumes the declared identity; only Compile rederives its digest.
    bytes[4] ^= 1;
    let decoded = decode_canonical::<
        DecodedCborIdentityRecord<PersistentTypeId, DecodedSourceDeclarationKey>,
    >(&bytes)
    .unwrap();
    let mut pending = PendingIdentityValidation::for_link();
    pending.register_authority(ConeIdentity::CORE).unwrap();
    pending.register(IdentityLayer::Hir, &decoded).unwrap();
    pending.resolve(&decoded).unwrap();
    let mut graph = pending.finish().unwrap();
    let id = PersistentIdResolver::resolve(&mut graph, decoded.decoded_id()).unwrap();
    assert_eq!(id.as_array(), decoded.decoded_id().as_array());
    assert_eq!(
        graph
            .canonical_key::<_, SourceDeclarationKey>(id)
            .unwrap()
            .as_ref(),
        source_type_record().key()
    );
    let wrong_kind = DecodedPersistentId::<PersistentFunctionId>::from_bytes(*id.as_array());
    assert!(PersistentIdResolver::resolve(&mut graph, wrong_kind).is_err());

    let exact = decoded_exact_type();
    let mut missing = PendingIdentityValidation::for_link();
    missing.register(IdentityLayer::Mir, &exact).unwrap();
    assert!(missing.resolve(&exact).is_err());
}
