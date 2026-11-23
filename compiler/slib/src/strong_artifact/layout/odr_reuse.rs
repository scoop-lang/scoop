//! Omit complete ODR objects already stored by an actual dependency.

use std::collections::{BTreeMap, BTreeSet};

use scoop_identity::{OdrGroupId, OdrMemberId};
use scoop_wire::Digest256;

use crate::{
    CanonicalDefinedLinkSymbolOwnerSetV1, CanonicalOdrMemberDirectoryV1, LinkDefinitionOwnerV1,
    LinkIdentityClosureSectionV1, OdrMemberDirectoryEntryV1, SlibMemberId,
    UncompressedObjectMember, ValidatedCrossConeStrongLinkArtifact,
};

type OdrBundle = Vec<(OdrGroupId, OdrMemberDirectoryEntryV1)>;

/// Only the dependency definitions and complete ODR objects needed by publication.
pub struct CrossConePublicationDependencies {
    owners: Vec<CanonicalDefinedLinkSymbolOwnerSetV1>,
    available: BTreeMap<Digest256, Vec<OdrBundle>>,
}

impl CrossConePublicationDependencies {
    pub(crate) fn from_links(links: Vec<ValidatedCrossConeStrongLinkArtifact>) -> Self {
        let mut available = BTreeMap::<Digest256, Vec<OdrBundle>>::new();
        let mut owners = Vec::with_capacity(links.len());
        for dependency in links {
            let mut bundles = bundles(
                dependency.defined_symbols(),
                dependency.definition_data().odr_members(),
            );
            for record in dependency.member_records() {
                if let Some(bundle) = bundles.remove(&record.id()) {
                    let candidates = available.entry(record.sha256()).or_default();
                    if !candidates.contains(&bundle) {
                        candidates.push(bundle);
                    }
                }
            }
            owners.push(dependency.into_definition_data().into_defined_symbols());
        }
        Self { owners, available }
    }

    pub fn symbol_owners(&self) -> &[CanonicalDefinedLinkSymbolOwnerSetV1] {
        &self.owners
    }

    pub(super) fn select(
        &self,
        objects: &[UncompressedObjectMember],
        link: &LinkIdentityClosureSectionV1,
        odr: &CanonicalOdrMemberDirectoryV1,
    ) -> BTreeSet<SlibMemberId> {
        if self.available.is_empty() {
            return BTreeSet::new();
        }
        let bundles = bundles(link.defined_symbols(), odr);
        objects
            .iter()
            .filter_map(|object| {
                let record = object.content();
                let bundle = bundles.get(&record.id())?;
                let candidates = self.available.get(&record.sha256())?;
                candidates.contains(bundle).then_some(record.id())
            })
            .collect()
    }
}

fn bundles(
    defined: &CanonicalDefinedLinkSymbolOwnerSetV1,
    odr: &CanonicalOdrMemberDirectoryV1,
) -> BTreeMap<SlibMemberId, OdrBundle> {
    let entries = odr
        .groups()
        .iter()
        .flat_map(|group| {
            group
                .members()
                .iter()
                .map(|entry| (entry.member(), (group.group(), *entry)))
        })
        .collect::<BTreeMap<_, _>>();
    let mut objects = BTreeMap::<SlibMemberId, BTreeSet<OdrMemberId>>::new();
    let mut excluded = BTreeSet::new();
    for definition in defined.owners() {
        match definition.owner() {
            LinkDefinitionOwnerV1::OdrDefinition(member) if entries.contains_key(&member) => {
                objects
                    .entry(definition.member())
                    .or_default()
                    .insert(member);
            }
            LinkDefinitionOwnerV1::VerifierBoundary { .. } => {}
            _ => {
                excluded.insert(definition.member());
            }
        }
    }
    objects
        .into_iter()
        .filter(|(member, _)| !excluded.contains(member))
        .map(|(member, entries_in_object)| {
            let bundle = entries_in_object
                .into_iter()
                .map(|id| entries[&id])
                .collect();
            (member, bundle)
        })
        .collect()
}
