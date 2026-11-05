//! Check full site identities only for the selected physical callable bodies.

use super::*;
use scoop_identity::{
    PersistentCallableBodyId, PersistentSafepointSiteId, StrongDefinitionEntityKind,
};

impl SelectedMembers {
    pub(super) fn check_safepoints(&self, closure: &ProgramLinkClosure) -> Result<(), LinkError> {
        let mut ids = BTreeMap::new();
        let mut owners = BTreeMap::new();
        for (artifact, _) in closure.artifacts() {
            let production = artifact.production();
            for id in self.definitions(artifact.identity()) {
                let plan = &production.definitions[id];
                if plan.role != StrongDefinitionRole::CallableBody {
                    continue;
                }
                let StrongDefinitionEntityKind::CallableBody(owner) = plan.entity.kind() else {
                    return Err(error(
                        "selected callable definition has a non-callable entity",
                    ));
                };
                if let Some(sites) = production.safepoints.get(&owner) {
                    for site in sites {
                        insert_site(&mut ids, &mut owners, owner, *site)?;
                    }
                }
            }
        }
        Ok(())
    }
}

fn insert_site(
    ids: &mut BTreeMap<u64, PersistentSafepointSiteId>,
    owners: &mut BTreeMap<PersistentSafepointSiteId, PersistentCallableBodyId>,
    owner: PersistentCallableBodyId,
    site: scoop_lir::ProgramLinkSafepoint,
) -> Result<(), LinkError> {
    let id = site.id.get();
    if let Some(previous) = ids.insert(id, site.site)
        && previous != site.site
    {
        return Err(error(format!(
            "selected safepoints {previous} and {} share runtime ID {id}",
            site.site
        )));
    }
    if let Some(previous) = owners.insert(site.site, owner)
        && previous != owner
    {
        return Err(error(format!(
            "selected safepoint {} has owners {previous} and {owner}",
            site.site
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use scoop_identity::{
        CallableBodyKey, CanonicalIdentifier, DeclarationScope, DefinitionOwnerChain, PackagePath,
        PersistentFunctionId, SafepointId, SafepointSiteKey, SafepointSiteRole,
        SourceDeclarationKey, SourceDeclarationSite, StrongCallableDefinitionOwner,
    };

    fn body(name: &str) -> PersistentCallableBodyId {
        let scope = SourceDeclarationSite::new(
            ConeIdentity::SINGLE_FILE,
            PackagePath::root(),
            DefinitionOwnerChain::top_level(),
            DeclarationScope::ConeWide,
        )
        .unwrap();
        let function =
            PersistentFunctionId::from_source_declaration(&SourceDeclarationKey::function(
                scope,
                CanonicalIdentifier::new(name).unwrap(),
                0,
                None,
                Vec::new(),
            ))
            .unwrap();
        PersistentCallableBodyId::from_key(&CallableBodyKey::strong(
            StrongCallableDefinitionOwner::Function(function),
        ))
        .unwrap()
    }

    fn site(owner: PersistentCallableBodyId, ordinal: u32) -> scoop_lir::ProgramLinkSafepoint {
        let site = PersistentSafepointSiteId::from_key(&SafepointSiteKey::new(
            owner,
            SafepointSiteRole::ManagedPoll,
            ordinal,
        ))
        .unwrap();
        scoop_lir::ProgramLinkSafepoint {
            site,
            id: SafepointId::derive(site).unwrap(),
        }
    }

    #[test]
    fn selected_sites_reject_short_id_collisions_and_conflicting_owners() {
        let owner = body("selected");
        let first = site(owner, 0);
        let second = scoop_lir::ProgramLinkSafepoint {
            id: first.id,
            ..site(owner, 1)
        };
        let mut ids = BTreeMap::new();
        let mut owners = BTreeMap::new();
        insert_site(&mut ids, &mut owners, owner, first).unwrap();
        assert!(
            insert_site(&mut ids, &mut owners, owner, second)
                .unwrap_err()
                .to_string()
                .contains("share runtime ID")
        );
        let mut ids = BTreeMap::new();
        let mut owners = BTreeMap::new();
        insert_site(&mut ids, &mut owners, owner, first).unwrap();
        assert!(
            insert_site(&mut ids, &mut owners, body("other"), first)
                .unwrap_err()
                .to_string()
                .contains("has owners")
        );
    }

    #[test]
    fn selected_sites_keep_distinct_slots_and_allow_the_same_identity_reference() {
        let owner = body("selected");
        let mut ids = BTreeMap::new();
        let mut owners = BTreeMap::new();
        for value in [site(owner, 0), site(owner, 1), site(owner, 0)] {
            insert_site(&mut ids, &mut owners, owner, value).unwrap();
        }
        assert_eq!((ids.len(), owners.len()), (2, 2));
    }
}
