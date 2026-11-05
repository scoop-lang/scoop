//! Remove private site and body records eliminated by ordinary code optimization.

use super::*;
use scoop_identity::{PersistentSafepointSiteId, PersistentSymbolRequestTable};
use std::collections::BTreeSet;

impl ConeLirFoundation {
    pub(crate) fn without_codegen_records(
        &self,
        sites: &BTreeSet<PersistentSafepointSiteId>,
        atoms: &BTreeSet<ObjectDefinitionAtomId>,
    ) -> Self {
        if sites.is_empty() && atoms.is_empty() {
            return self.clone();
        }
        let mut result = self.clone();
        let canonical = Rc::make_mut(&mut result.canonical);
        canonical
            .safepoint_sites
            .retain(|record| !sites.contains(&record.id()));
        canonical
            .safepoints
            .retain(|record| !sites.contains(&record.site()));
        canonical
            .definition_atoms
            .retain(|record| !atoms.contains(&record.id()));
        if !atoms.is_empty() {
            result.atom_index = Rc::new(DefinitionAtomIndex::new(&canonical.definition_atoms));
        }
        let requests = canonical
            .symbol_requests
            .requests()
            .iter()
            .copied()
            .filter(|request| match request.key() {
                PersistentSymbolKey::DefinitionBoundaryStart(atom)
                | PersistentSymbolKey::DefinitionBoundaryEnd(atom) => !atoms.contains(&atom),
                _ => true,
            })
            .collect();
        canonical.symbol_requests = PersistentSymbolRequestTable::new(requests)
            .expect("retaining a subset preserves unique symbol requests");
        result
    }
}
