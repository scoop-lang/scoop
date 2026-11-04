use super::{StrongSafepointProductionPlanBuildError, StrongSafepointProductionPlanSetV1};
use scoop_identity::{ConeIdentity, DigestNodeKey};
use std::collections::{BTreeMap, BTreeSet};
mod support;
use support::Fixture;

#[test]
fn keeps_site_semantics_and_machine_content_nodes_without_runtime_definitions() {
    let fixture = Fixture::new();
    let plan = fixture.build().unwrap();
    assert_eq!(plan.producer(), ConeIdentity::SINGLE_FILE);
    assert_eq!(plan.sites().len(), 2);
    for (site, semantic) in plan.sites().iter().zip(fixture.semantics.sites()) {
        assert_eq!(
            (
                site.site(),
                site.safepoint(),
                site.owner(),
                site.role(),
                site.root_pair_count()
            ),
            (
                semantic.site(),
                semantic.safepoint(),
                semantic.owner(),
                semantic.role(),
                semantic.root_pair_count()
            )
        );
        assert_eq!(
            site.normalized_stackmap_fingerprint_node(),
            fixture
                .digests
                .node(&DigestNodeKey::stackmap_record(site.site()))
                .unwrap()
                .id()
        );
    }
}

#[test]
fn final_emission_removes_dead_sites_and_updates_physical_roots() {
    let fixture = Fixture::new();
    let mut plan = fixture.build().unwrap();
    let retained = plan.sites()[0];
    plan.set_emitted_root_counts(&BTreeMap::from([(retained.safepoint(), 3)]))
        .unwrap();
    assert_eq!(plan.sites().len(), 1);
    assert_eq!(plan.sites()[0].site(), retained.site());
    assert_eq!(plan.sites()[0].root_pair_count(), 3);
    plan.set_emitted_root_counts(&BTreeMap::new()).unwrap();
    assert!(plan.sites().is_empty());
}

#[test]
fn rejects_an_emitted_site_outside_the_semantic_plan() {
    let fixture = Fixture::new();
    let mut plan = fixture.build().unwrap();
    let unknown = crate::SafepointIdentity::new(
        plan.sites()[0].owner(),
        crate::SafepointSiteRole::ManagedPoll,
        99,
    )
    .unwrap();
    assert_eq!(
        plan.set_emitted_root_counts(&BTreeMap::from([(unknown.runtime_id(), 0)])),
        Err(StrongSafepointProductionPlanBuildError::EmittedSiteSet)
    );
}

#[test]
fn removed_site_content_updates_digest_edges_and_requires_a_production_node() {
    let mut fixture = Fixture::new();
    let removed = fixture.digests.nodes()[0].clone();
    fixture
        .digests
        .remove_safepoint_records(&BTreeSet::from([removed.id()]));
    fixture
        .digests
        .validate_against(&fixture.foundation)
        .unwrap();
    assert!(
        matches!(fixture.build(), Err(StrongSafepointProductionPlanBuildError::MissingDigestNode(key)) if &key == removed.key())
    );
}
