//! Merge already resolved dependency nodes without rebuilding lookup indexes.

use super::*;

/// A dependency's own records, sharing the existing lookup storage.
#[derive(Clone)]
pub(super) struct IdentitySource {
    candidates: Rc<CandidateMap>,
    nodes: Rc<[IdentityNode]>,
    declared: Rc<[IdentityNode]>,
}

impl IdentitySource {
    pub(super) fn pointer(&self) -> *const CandidateMap {
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
        // Most readers have only registered their own Cone before the first import.
        // Share the dependency's compact index and overlay those local records.
        let shared_base =
            self.external_sources.is_empty() && self.candidates.len() < graph.candidates.len();
        if shared_base {
            let mut candidates = graph.candidates.as_ref().clone();
            for (node, existing) in self.candidates.iter() {
                #[cfg(test)]
                {
                    self.external_candidate_visits += 1;
                }
                let mut missing_key = None;
                if !self.local_layers.contains_key(node)
                    && let Some(source) = candidates.get(node)
                {
                    if !existing.matches_external(source) {
                        conflict = Some(
                            conflict.map_or(*node, |previous: IdentityNode| previous.min(*node)),
                        );
                    } else if existing.canonical_key.is_none() {
                        missing_key.clone_from(&source.canonical_key);
                    }
                }
                candidates.insert_shared(node, &self.candidates);
                if let Some(key) = missing_key {
                    candidates.set_key(node, key);
                }
            }
            self.candidates = candidates;
        }
        let local = IdentitySource {
            candidates: Rc::clone(&graph.candidates),
            nodes: Rc::clone(&graph.local_nodes),
            declared: Rc::clone(&graph.declared),
        };
        for source in graph.external_sources.iter().chain(std::iter::once(&local)) {
            if !self.imported_sources.insert(source.pointer()) {
                continue;
            }
            if !shared_base {
                for &node in source.nodes.iter().chain(source.declared.iter()) {
                    #[cfg(test)]
                    {
                        self.external_candidate_visits += 1;
                    }
                    let record = source
                        .candidates
                        .get(&node)
                        .expect("source owns the listed identity");
                    if let Some(candidate) = self.candidates.get(&node) {
                        if self.local_layers.contains_key(&node) {
                            continue;
                        }
                        if !candidate.matches_external(record) {
                            conflict = Some(
                                conflict.map_or(node, |previous: IdentityNode| previous.min(node)),
                            );
                        } else if candidate.canonical_key.is_none()
                            && let Some(key) = &record.canonical_key
                        {
                            self.candidates.set_key(&node, Arc::clone(key));
                        }
                    } else {
                        self.candidates.insert_shared(&node, &source.candidates);
                    }
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
