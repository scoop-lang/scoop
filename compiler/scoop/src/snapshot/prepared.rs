use std::collections::BTreeMap;

use scoop_manifest::{LoadedConeManifest, select_manifest_sources, select_single_file_source};

use super::staging::PreparedStaging;
use crate::discovery::{BuildContext, GraphNode};
use crate::graph::ResolvedGraphParts;
use crate::locator::{PrebuiltArtifactCandidate, PrebuiltArtifactProjection};
use crate::{ResolvedBuildGraph, ResolvedPairedScoopc};

mod documented_execution;
mod error;
mod model;
mod observation;
mod ordinary_execution;

pub use error::PrepareBuildGraphError;
pub use observation::{BuildDumpRequest, DumpScope};
pub use ordinary_execution::OrdinarySourceExecutionError;
pub(crate) use ordinary_execution::{PendingSourceExecution, PreparedSourceExecution};

pub use model::{
    ChildRequestPlanError, CompileCacheKeyError, ManifestSourceInput, PreparedArtifactCandidate,
    PreparedBuildGraph, PreparedNodeRepresentation, SelectedSource, SingleFileSourceInput,
};

use model::{
    NonEmptyPreparedArtifactCandidates, PreparedGraphNode, PreparedManifestSourceNode,
    PreparedPrebuiltArtifactNode, PreparedSingleFileNode, SourceNativeInput,
};

impl ResolvedBuildGraph {
    /// Prepares the graph and actual input paths for compiler child execution.
    pub fn prepare(self) -> Result<PreparedBuildGraph, PrepareBuildGraphError> {
        Preparer::new(self.into_parts(), false).run()
    }

    pub(crate) fn prepare_frontend(self) -> Result<PreparedBuildGraph, PrepareBuildGraphError> {
        Preparer::new(self.into_parts(), true).run()
    }
}

impl PreparedBuildGraph {
    pub(crate) fn prepare_machine_source(
        &mut self,
        identity: scoop_identity::ConeIdentity,
    ) -> Result<(), PrepareBuildGraphError> {
        let Some(PreparedGraphNode::ManifestSource(node)) = self.nodes.get_mut(&identity) else {
            return Ok(());
        };
        if matches!(node.snapshot.native, SourceNativeInput::Artifact(_)) {
            return Ok(());
        }
        let native = scoop_toolchain::prepare_native_objects(
            &node.manifest,
            self.context.target.lir_target(),
            || self.context.target.production(),
            self.context.optimization,
            self.context.sysroot.as_path(),
            self.context.cache_root.as_path(),
            if node.snapshot.test_config.is_some() {
                scoop_protocol::CompilationPurpose::Test
            } else {
                scoop_protocol::CompilationPurpose::Production
            },
        )
        .map_err(PrepareBuildGraphError::Native)?;
        node.snapshot.native = SourceNativeInput::Artifact(native);
        Ok(())
    }
}

struct Preparer {
    parts: ResolvedGraphParts,
    staging: PreparedStaging,
    compiler: ResolvedPairedScoopc,
    frontend: bool,
}

impl Preparer {
    fn new(parts: ResolvedGraphParts, frontend: bool) -> Self {
        let compiler = ResolvedPairedScoopc::resolve(&parts.context.compiler);
        let staging = PreparedStaging::new(&parts.context.cache_root);
        Self {
            parts,
            staging,
            compiler,
            frontend,
        }
    }

    fn run(self) -> Result<PreparedBuildGraph, PrepareBuildGraphError> {
        let mut nodes = BTreeMap::new();

        for (identity, node) in self.parts.nodes {
            let prepared = match node {
                GraphNode::ManifestSource(manifest) => {
                    let purpose = if identity == self.parts.root {
                        self.parts.context.purpose
                    } else {
                        scoop_protocol::CompilationPurpose::Production
                    };
                    let snapshot = select_manifest_input(
                        &manifest,
                        &self.parts.context,
                        purpose,
                        self.frontend,
                    )?;
                    PreparedGraphNode::ManifestSource(Box::new(PreparedManifestSourceNode {
                        snapshot,
                        manifest,
                    }))
                }
                GraphNode::Prebuilt(prebuilt) => {
                    PreparedGraphNode::PrebuiltArtifact(Box::new(PreparedPrebuiltArtifactNode {
                        coordinate: prebuilt.coordinate().clone(),
                        artifact_fingerprint: prebuilt.artifact_fingerprint(),
                        candidates: prepare_prebuilt_candidates(*prebuilt),
                    }))
                }
                GraphNode::SingleFile(locator) => {
                    let discovered = select_single_file_source(&locator)
                        .map_err(PrepareBuildGraphError::SingleFile)?;
                    let snapshot = SingleFileSourceInput { source: discovered };
                    let input_path = locator.resolved_path().to_path_buf();
                    PreparedGraphNode::SingleFile(Box::new(PreparedSingleFileNode {
                        snapshot,
                        input_path,
                    }))
                }
            };
            nodes.insert(identity, prepared);
        }
        Ok(PreparedBuildGraph {
            root: self.parts.root,
            nodes,
            edges: self.parts.edges,
            dependency_first: self.parts.dependency_first,
            source_inputs: self.parts.source_inputs,
            root_kind: self.parts.root_kind,
            target_selection: self.parts.target_selection,
            context: self.parts.context,

            staging: self.staging,
            compiler: self.compiler,
            dumps: BTreeMap::new(),
        })
    }
}

fn select_manifest_input(
    manifest: &LoadedConeManifest,
    context: &BuildContext,
    purpose: scoop_protocol::CompilationPurpose,
    frontend: bool,
) -> Result<ManifestSourceInput, PrepareBuildGraphError> {
    let discovered = match purpose {
        scoop_protocol::CompilationPurpose::Production => {
            select_manifest_sources(manifest, context.target.id())
        }
        scoop_protocol::CompilationPurpose::Test => {
            scoop_manifest::select_manifest_sources_for_tests(manifest, context.target.id())
        }
    }
    .map_err(PrepareBuildGraphError::SourceDiscovery)?;

    let coordinate = manifest.coordinate().clone();
    let identity = manifest.identity();
    let requested_kind = match purpose {
        scoop_protocol::CompilationPurpose::Production => {
            manifest.parsed().semantic().requested_kind()
        }
        scoop_protocol::CompilationPurpose::Test => scoop_identity::RequestedConeKind::Executable,
    };
    let test_config = (purpose == scoop_protocol::CompilationPurpose::Test)
        .then(|| manifest.parsed().tests().clone());
    let manifest_semantic = manifest.parsed().semantic().clone();
    let native = if frontend {
        SourceNativeInput::Frontend
    } else {
        SourceNativeInput::Artifact(
            scoop_toolchain::prepare_native_objects(
                manifest,
                context.target.lir_target(),
                || context.target.production(),
                context.optimization,
                context.sysroot.as_path(),
                context.cache_root.as_path(),
                purpose,
            )
            .map_err(PrepareBuildGraphError::Native)?,
        )
    };
    Ok(ManifestSourceInput {
        identity,
        coordinate,
        requested_kind,
        manifest_semantic,
        test_config,
        native,
        manifest_locator: manifest.manifest_path().to_path_buf(),
        input_root: manifest.input_root().to_path_buf(),
        sources: discovered,
    })
}

fn prepare_prebuilt_candidates(
    prebuilt: PrebuiltArtifactProjection,
) -> NonEmptyPreparedArtifactCandidates {
    let (_, _, first, rest) = prebuilt.into_parts();
    NonEmptyPreparedArtifactCandidates {
        first: prepare_artifact_candidate(first),
        rest: rest.into_iter().map(prepare_artifact_candidate).collect(),
    }
}

fn prepare_artifact_candidate(candidate: PrebuiltArtifactCandidate) -> PreparedArtifactCandidate {
    PreparedArtifactCandidate {
        artifact: candidate.into_artifact(),
    }
}

#[cfg(test)]
mod tests;
