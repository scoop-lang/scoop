//! Canonical record queries over local layers and their resolved dependencies.

use super::*;

#[derive(Clone, Copy)]
pub(super) enum RecordScope {
    Layer(IdentityLayer),
    Closure,
}

impl RecordScope {
    fn includes(self, layer: Option<IdentityLayer>) -> bool {
        match self {
            Self::Layer(expected) => layer == Some(expected),
            Self::Closure => true,
        }
    }
}

impl ValidatedIdentityGraph {
    pub(super) fn record_candidates(
        &self,
        kind: &'static str,
        scope: RecordScope,
    ) -> impl Iterator<Item = (&IdentityNode, &Candidate)> + Clone {
        let local = match scope {
            RecordScope::Layer(_) => {
                let start = self.declared.partition_point(|node| node.kind < kind);
                let end = self.declared.partition_point(|node| node.kind <= kind);
                Some(self.declared[start..end].iter())
            }
            RecordScope::Closure => None,
        };
        let closure = match scope {
            RecordScope::Closure => Some(self.candidates.iter()),
            RecordScope::Layer(_) => None,
        };
        local
            .into_iter()
            .flatten()
            .map(|node| (node, &self.candidates[node]))
            .chain(
                closure
                    .into_iter()
                    .flatten()
                    .filter(move |(node, _)| node.kind == kind),
            )
            .filter(move |(_, candidate)| scope.includes(candidate.layer))
    }

    /// Reconstructs this artifact's records from one layer, sorted by typed id.
    pub fn records<I, K>(
        &self,
        layer: IdentityLayer,

        path: &WirePath,
    ) -> Result<Vec<CborIdentityRecord<I, K>>, IdentityValidationError>
    where
        I: PersistentId + 'static,
        K: CborIdentityKey<I> + Send + Sync + 'static,
    {
        self.records_in_scope(RecordScope::Layer(layer), path)
    }

    /// Borrows canonical keys from this artifact and its resolved dependency
    /// closure. Imported records keep their ownership and are not redeclared.
    pub fn closure_records<I, K>(
        &self,

        path: &WirePath,
    ) -> Result<Vec<CborIdentityRecord<I, K>>, IdentityValidationError>
    where
        I: PersistentId + 'static,
        K: CborIdentityKey<I> + Send + Sync + 'static,
    {
        self.records_in_scope(RecordScope::Closure, path)
    }

    fn records_in_scope<I, K>(
        &self,
        scope: RecordScope,

        path: &WirePath,
    ) -> Result<Vec<CborIdentityRecord<I, K>>, IdentityValidationError>
    where
        I: PersistentId + 'static,
        K: CborIdentityKey<I> + Send + Sync + 'static,
    {
        let candidates = self.record_candidates(I::KIND, scope);
        let record_count = candidates
            .clone()
            .filter(|(_, candidate)| candidate.key_ref::<I, K>().is_some())
            .count();
        let record_count = u64::try_from(record_count).map_err(|_| {
            IdentityValidationError::Resource(WireError::new(
                WireErrorKind::IntegerOutOfRange,
                path.clone(),
                None,
            ))
        })?;
        let mut records = Vec::new();
        scoop_wire::allocation::try_reserve_count(&mut records, record_count, path)
            .map_err(IdentityValidationError::Resource)?;
        for (node, candidate) in candidates {
            let Some(key) = candidate.key::<I, K>() else {
                continue;
            };
            let Some(id) = candidate.trusted_id.downcast_ref::<I>().copied() else {
                return Err(IdentityValidationError::InvalidRecord {
                    kind: I::KIND,
                    id: node.bytes,
                    reason: "validated identity has the wrong concrete id type".to_owned(),
                });
            };
            let record = CborIdentityRecord::from_verified_shared(id, key);
            records.push(record);
        }
        records.sort_by_key(CborIdentityRecord::id);
        Ok(records)
    }
}
