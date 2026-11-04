//! Semantic safepoint production data and normalized machine-record fingerprints.

pub(in crate::link_object) mod fingerprints;
pub use fingerprints::*;

mod verification;
pub use verification::*;
