//! Coverage and actual consumers of one feedback compilation.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FeedbackConsumer {
    Inline,
    StringFusion,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UseSummary {
    pub matched_functions: usize,
    pub matched_positions: usize,
    pub partial_positions: usize,
    pub missing_positions: usize,
    pub changed_positions: usize,
    pub consumers: BTreeMap<FeedbackConsumer, usize>,
}

impl UseSummary {
    pub fn used(&self) -> bool {
        self.consumers.values().any(|&positions| positions != 0)
    }
}
