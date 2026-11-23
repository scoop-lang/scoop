//! Compact identity lookup slots sharing their immutable records across graphs.

use std::borrow::Borrow;
use std::hash::{Hash, Hasher};
use std::ops::Index;

use super::*;

#[derive(Clone)]
struct IdentityRecord {
    node: IdentityNode,
    candidate: Candidate,
}

#[derive(Clone)]
struct SharedRecord(Rc<IdentityRecord>);

impl Borrow<IdentityNode> for SharedRecord {
    fn borrow(&self) -> &IdentityNode {
        &self.0.node
    }
}

impl PartialEq for SharedRecord {
    fn eq(&self, other: &Self) -> bool {
        self.0.node == other.0.node
    }
}

impl Eq for SharedRecord {}

impl Hash for SharedRecord {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.node.hash(state);
    }
}

#[derive(Clone, Default)]
pub(super) struct CandidateMap {
    // Each slot contains one pointer, rather than an inline identity and record.
    records: Rc<HashSet<SharedRecord>>,
}

impl CandidateMap {
    pub(super) fn len(&self) -> usize {
        self.records.len()
    }

    pub(super) fn contains_key(&self, node: &IdentityNode) -> bool {
        self.records.contains(node)
    }

    pub(super) fn get(&self, node: &IdentityNode) -> Option<&Candidate> {
        self.records.get(node).map(|record| &record.0.candidate)
    }

    pub(super) fn iter(&self) -> impl Iterator<Item = (&IdentityNode, &Candidate)> + Clone {
        self.records
            .iter()
            .map(|record| (&record.0.node, &record.0.candidate))
    }

    pub(super) fn insert(&mut self, node: IdentityNode, candidate: Candidate) {
        Rc::make_mut(&mut self.records)
            .replace(SharedRecord(Rc::new(IdentityRecord { node, candidate })));
    }

    pub(super) fn insert_shared(&mut self, node: &IdentityNode, source: &Self) {
        let record = source.records.get(node).expect("source owns identity");
        Rc::make_mut(&mut self.records).replace(record.clone());
    }

    pub(super) fn set_key(&mut self, node: &IdentityNode, key: Arc<dyn ErasedCanonicalKey>) {
        let records = Rc::make_mut(&mut self.records);
        let mut record = records.take(node).expect("registered identity");
        Rc::make_mut(&mut record.0).candidate.canonical_key = Some(key);
        records.insert(record);
    }

    #[cfg(test)]
    pub(super) fn ptr_eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.records, &other.records)
    }
}

impl Index<&IdentityNode> for CandidateMap {
    type Output = Candidate;

    fn index(&self, node: &IdentityNode) -> &Self::Output {
        self.get(node).expect("registered identity")
    }
}
