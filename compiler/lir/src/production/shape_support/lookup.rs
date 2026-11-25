//! Temporary shape lookups, independent of the canonical record order.

use scoop_identity::{
    PersistentExactTypeId, PersistentLayoutId, PersistentScanId, RepresentationRole, ScanRole,
};

use crate::ConeLirFoundation;

pub(super) struct ShapeLookup {
    layouts: Vec<(PersistentExactTypeId, PersistentLayoutId)>,
    scans: Vec<(PersistentLayoutId, PersistentScanId)>,
}

impl ShapeLookup {
    pub(super) fn new(foundation: &ConeLirFoundation) -> Self {
        let mut layouts = foundation
            .layouts()
            .iter()
            .filter(|record| record.key().representation() == RepresentationRole::ManagedValue)
            .map(|record| (record.key().exact_type(), record.id()))
            .collect::<Vec<_>>();
        layouts.sort_unstable();
        let mut scans = foundation
            .scans()
            .iter()
            .filter(|record| record.key().role() == ScanRole::InlineValue)
            .map(|record| (record.key().layout(), record.id()))
            .collect::<Vec<_>>();
        scans.sort_unstable();
        Self { layouts, scans }
    }

    pub(super) fn layouts(
        &self,
        exact: PersistentExactTypeId,
    ) -> &[(PersistentExactTypeId, PersistentLayoutId)] {
        let start = self.layouts.partition_point(|(key, _)| *key < exact);
        let end = self.layouts[start..].partition_point(|(key, _)| *key == exact);
        &self.layouts[start..start + end]
    }

    pub(super) fn scans(
        &self,
        layout: PersistentLayoutId,
    ) -> &[(PersistentLayoutId, PersistentScanId)] {
        let start = self.scans.partition_point(|(key, _)| *key < layout);
        let end = self.scans[start..].partition_point(|(key, _)| *key == layout);
        &self.scans[start..start + end]
    }
}
