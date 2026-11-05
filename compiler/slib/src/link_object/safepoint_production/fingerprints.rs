use std::fmt;

use super::SafepointProductionDataV1;
use crate::link_object::StackmapRecordFingerprintV1;
use scoop_identity::{DigestNodeId, PersistentSafepointSiteId};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VerifiedStrongSafepointFingerprintV1 {
    site: PersistentSafepointSiteId,

    stackmap_node: DigestNodeId,
    stackmap: StackmapRecordFingerprintV1,
}

impl VerifiedStrongSafepointFingerprintV1 {
    pub const fn site(self) -> PersistentSafepointSiteId {
        self.site
    }

    pub const fn stackmap_node(self) -> DigestNodeId {
        self.stackmap_node
    }

    pub const fn stackmap(self) -> StackmapRecordFingerprintV1 {
        self.stackmap
    }
}

/// Stackmap fingerprints from the already normalized machine-site records.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedStrongSafepointFingerprintSetV1 {
    production: SafepointProductionDataV1,
    fingerprints: Vec<VerifiedStrongSafepointFingerprintV1>,
}

impl VerifiedStrongSafepointFingerprintSetV1 {
    pub const fn producer(&self) -> scoop_identity::ConeIdentity {
        self.production.producer()
    }

    pub const fn production(&self) -> &SafepointProductionDataV1 {
        &self.production
    }

    pub fn fingerprints(&self) -> &[VerifiedStrongSafepointFingerprintV1] {
        &self.fingerprints
    }
}

pub fn compute_strong_safepoint_fingerprints_v1(
    production: SafepointProductionDataV1,
) -> Result<VerifiedStrongSafepointFingerprintSetV1, StrongSafepointFingerprintError> {
    let planned = production.plan().sites();
    let stackmaps = production.stackmaps().records();
    let mut fingerprints = Vec::with_capacity(planned.len());
    for plan in planned {
        let index = stackmaps
            .binary_search_by_key(&plan.site(), |record| {
                record.normalized().canonical().site()
            })
            .map_err(|_| StrongSafepointFingerprintError::MissingStackmap(plan.site()))?;
        let stackmap = &stackmaps[index];
        let stackmap_fingerprint = stackmap.normalized().fingerprint();
        fingerprints.push(VerifiedStrongSafepointFingerprintV1 {
            site: plan.site(),
            stackmap_node: plan.normalized_stackmap_fingerprint_node(),
            stackmap: stackmap_fingerprint,
        });
    }

    Ok(VerifiedStrongSafepointFingerprintSetV1 {
        production,
        fingerprints,
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StrongSafepointFingerprintError {
    MissingStackmap(PersistentSafepointSiteId),
}

impl fmt::Display for StrongSafepointFingerprintError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "failed to compute strong safepoint fingerprints: {self:?}"
        )
    }
}

impl std::error::Error for StrongSafepointFingerprintError {}
