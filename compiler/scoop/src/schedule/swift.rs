use std::collections::{BTreeMap, VecDeque};

use scoop_identity::ConeIdentity;
use scoop_toolchain::swift::{SwiftBuildRequest, SwiftProduct};

use super::{BuildGraphExecutionError, PreparedBuildGraph};

pub(super) struct SwiftExecution {
    pub(super) requests: Vec<(ConeIdentity, SwiftBuildRequest)>,
    pub(super) ready: VecDeque<usize>,
    pub(super) completed: BTreeMap<usize, SwiftProduct>,
}

impl SwiftExecution {
    pub(super) fn new(graph: &PreparedBuildGraph, include_root: bool) -> Self {
        let requests = if include_root {
            graph.swift_requests().to_vec()
        } else {
            Vec::new()
        };
        Self {
            ready: (0..requests.len()).collect(),
            requests,
            completed: BTreeMap::new(),
        }
    }

    pub(super) fn error(
        &self,
        position: usize,
        source: scoop_toolchain::ToolchainError,
    ) -> BuildGraphExecutionError {
        let (cone, request) = &self.requests[position];
        BuildGraphExecutionError::Swift {
            cone: *cone,
            product: request.product().to_owned(),
            source: Box::new(source),
        }
    }
}
