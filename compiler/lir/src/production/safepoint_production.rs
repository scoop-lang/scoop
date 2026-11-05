//! Final machine-site production data; sites have no separate runtime definition.

use crate::{
    DigestFinalizationPlanV1, StrongSafepointSemanticPlanSetV1, StrongSafepointSemanticPlanV1,
};
use scoop_identity::{
    ConeIdentity, DigestNodeId, DigestNodeKey, PersistentCallableBodyId, PersistentSafepointSiteId,
    SafepointId, SafepointSiteRole,
};
use std::{collections::BTreeMap, fmt};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StrongSafepointProductionPlanV1 {
    semantic: StrongSafepointSemanticPlanV1,
    normalized_stackmap_fingerprint_node: DigestNodeId,
}

impl StrongSafepointProductionPlanV1 {
    pub const fn site(self) -> PersistentSafepointSiteId {
        self.semantic.site()
    }
    pub const fn safepoint(self) -> SafepointId {
        self.semantic.safepoint()
    }
    pub const fn owner(self) -> PersistentCallableBodyId {
        self.semantic.owner()
    }
    pub const fn role(self) -> SafepointSiteRole {
        self.semantic.role()
    }
    pub const fn root_pair_count(self) -> u32 {
        self.semantic.root_pair_count()
    }
    pub const fn normalized_stackmap_fingerprint_node(self) -> DigestNodeId {
        self.normalized_stackmap_fingerprint_node
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StrongSafepointProductionPlanSetV1 {
    producer: ConeIdentity,
    sites: Vec<StrongSafepointProductionPlanV1>,
}

impl StrongSafepointProductionPlanSetV1 {
    pub fn new(
        semantics: &StrongSafepointSemanticPlanSetV1,
        digests: &DigestFinalizationPlanV1,
    ) -> Result<Self, StrongSafepointProductionPlanBuildError> {
        let sites = semantics
            .sites()
            .iter()
            .map(|semantic| {
                let key = DigestNodeKey::stackmap_record(semantic.site());
                let node = digests.node(&key).ok_or(
                    StrongSafepointProductionPlanBuildError::MissingDigestNode(key),
                )?;
                Ok(StrongSafepointProductionPlanV1 {
                    semantic: *semantic,
                    normalized_stackmap_fingerprint_node: node.id(),
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            producer: semantics.producer(),
            sites,
        })
    }

    pub(crate) fn set_emitted_root_counts(
        &mut self,
        counts: &BTreeMap<SafepointId, u32>,
    ) -> Result<(), StrongSafepointProductionPlanBuildError> {
        let known = self
            .sites
            .iter()
            .map(|plan| plan.safepoint())
            .collect::<std::collections::BTreeSet<_>>();
        if counts.keys().any(|site| !known.contains(site)) {
            return Err(StrongSafepointProductionPlanBuildError::EmittedSiteSet);
        }
        self.sites
            .retain(|plan| counts.contains_key(&plan.safepoint()));
        for plan in &mut self.sites {
            plan.semantic = StrongSafepointSemanticPlanV1::from_artifact(
                plan.site(),
                plan.safepoint(),
                plan.owner(),
                plan.role(),
                counts[&plan.safepoint()],
            );
        }
        Ok(())
    }
    pub const fn producer(&self) -> ConeIdentity {
        self.producer
    }
    pub fn sites(&self) -> &[StrongSafepointProductionPlanV1] {
        &self.sites
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StrongSafepointProductionPlanBuildError {
    EmittedSiteSet,
    MissingDigestNode(DigestNodeKey),
}
impl fmt::Display for StrongSafepointProductionPlanBuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid safepoint production: {self:?}")
    }
}
impl std::error::Error for StrongSafepointProductionPlanBuildError {}

#[cfg(test)]
mod tests;
