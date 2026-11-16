use super::*;

pub(super) struct Plan {
    pub graph: ResolvedBuildGraph,
    pub root: ConeCoordinate,
    pub sources: Vec<SelectedSource>,
}

impl Plan {
    pub fn new(graph: ResolvedBuildGraph, request: &CheckRequest) -> BuildResult<Self> {
        if request.purpose == CompilationPurpose::Test && graph.single_file_source().is_some() {
            return Err(BuildFailure::tool(
                scoop_source::DiagnosticCode::E2001,
                BuildFailurePhase::Request,
                "--tests requires a manifest Cone",
            ));
        }
        let selection =
            scoop_toolchain::select_target(request.target.canonical_triple()).map_err(|cause| {
                BuildFailure::tool(cause.code(), BuildFailurePhase::SourceSnapshot, &cause)
            })?;
        let root = graph
            .node_coordinate(graph.root_identity())
            .expect("resolved root exists")
            .clone();
        let mut sources = Vec::new();
        for id in graph.dependency_first() {
            let purpose = if *id == graph.root_identity() {
                request.purpose
            } else {
                CompilationPurpose::Production
            };
            if let Some(manifest) = graph.source_manifest(*id) {
                let selected = match purpose {
                    CompilationPurpose::Production => {
                        scoop_manifest::select_manifest_sources(manifest, selection.target().id())
                    }
                    CompilationPurpose::Test => scoop_manifest::select_manifest_sources_for_tests(
                        manifest,
                        selection.target().id(),
                    ),
                }
                .map_err(|cause| {
                    BuildFailure::tool(cause.code(), BuildFailurePhase::SourceSnapshot, &cause)
                        .at_host(cause.path())
                })?;
                sources.extend(selected.iter().cloned());
            } else if *id == graph.root_identity()
                && let Some(path) = graph.single_file_source()
            {
                let source = scoop_manifest::select_single_file_source(path).map_err(|cause| {
                    BuildFailure::tool(cause.code(), BuildFailurePhase::SourceSnapshot, &cause)
                })?;
                sources.push(source);
            }
        }
        Ok(Self {
            graph,
            root,
            sources,
        })
    }
}
