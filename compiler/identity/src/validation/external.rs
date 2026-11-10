//! Merge already resolved dependency nodes without rebuilding lookup indexes.

use super::*;

/// A dependency's own records, sharing the existing lookup storage.
#[derive(Clone)]
pub(super) struct IdentitySource {
    candidates: Rc<HashMap<IdentityNode, Candidate>>,
    nodes: Rc<[IdentityNode]>,
    declared: Rc<[IdentityNode]>,
}

impl IdentitySource {
    pub(super) fn pointer(&self) -> *const HashMap<IdentityNode, Candidate> {
        Rc::as_ptr(&self.candidates)
    }
}

impl PendingIdentityValidation {
    /// Imports resolved dependency entities while preserving local declarations.
    /// Canonical keys remain shared; only merge conflicts require comparison.
    pub fn register_external_graph_authorities(
        &mut self,
        graph: &ValidatedIdentityGraph,
    ) -> Result<(), IdentityValidationError> {
        self.require_registration_phase()?;
        let mut conflict = None;
        let local = IdentitySource {
            candidates: Rc::clone(&graph.candidates),
            nodes: Rc::clone(&graph.local_nodes),
            declared: Rc::clone(&graph.declared),
        };
        for source in graph.external_sources.iter().chain(std::iter::once(&local)) {
            if !self.imported_sources.insert(source.pointer()) {
                continue;
            }
            for &node in source.nodes.iter().chain(source.declared.iter()) {
                #[cfg(test)]
                {
                    self.external_candidate_visits += 1;
                }
                let source = &source.candidates[&node];
                if let Some(candidate) = self.candidates.get_mut(&node) {
                    if candidate.layer.is_some() {
                        continue;
                    }
                    let matches = candidate.trusted_id.as_ref().type_id()
                        == source.trusted_id.as_ref().type_id()
                        && match (&candidate.canonical_key, &source.canonical_key) {
                            (Some(existing), Some(key)) => {
                                Arc::ptr_eq(existing, key) || existing.equals(key.as_ref())
                            }
                            _ => true,
                        };
                    if !matches {
                        conflict = Some(
                            conflict.map_or(node, |previous: IdentityNode| previous.min(node)),
                        );
                    } else if candidate.canonical_key.is_none() {
                        candidate.canonical_key.clone_from(&source.canonical_key);
                    }
                } else {
                    self.reserve_candidate_slot()?;
                    let mut candidate = source.clone();
                    candidate.layer = None;
                    self.candidates.insert(node, candidate);
                }
            }
            self.external_sources.push(source.clone());
        }
        if let Some(node) = conflict {
            return self.fail(IdentityValidationError::IdentityCollision {
                kind: node.kind,
                id: node.bytes,
            });
        }
        Ok(())
    }
}
