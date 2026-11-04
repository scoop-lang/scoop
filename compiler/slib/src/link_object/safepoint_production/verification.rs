//! Safepoint plans and the actual normalized stackmap records.

use crate::link_object::ScoopStackmapSetV1;
use scoop_lir::StrongSafepointProductionPlanSetV1;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SafepointProductionDataV1 {
    stackmaps: ScoopStackmapSetV1,
    plan: StrongSafepointProductionPlanSetV1,
}
impl SafepointProductionDataV1 {
    pub const fn producer(&self) -> scoop_identity::ConeIdentity {
        self.plan.producer()
    }
    pub const fn plan(&self) -> &StrongSafepointProductionPlanSetV1 {
        &self.plan
    }
    pub const fn stackmaps(&self) -> &ScoopStackmapSetV1 {
        &self.stackmaps
    }
}

pub fn bind_safepoint_production_v1(
    stackmaps: ScoopStackmapSetV1,
    plan: StrongSafepointProductionPlanSetV1,
) -> SafepointProductionDataV1 {
    SafepointProductionDataV1 { stackmaps, plan }
}
