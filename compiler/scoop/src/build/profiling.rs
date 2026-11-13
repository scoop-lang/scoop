use super::*;

pub(super) fn collect(graph: &crate::BuildGraphOutcome) -> BuildResult<ProfilingBuildOutput> {
    let mut output = ProfilingBuildOutput::default();
    for node in graph.artifacts() {
        if node.artifact().summary().instrumentation() == scoop_profile::Instrumentation::Off {
            output
                .uninstrumented_cones
                .push(node.artifact().summary().cone().coordinate().to_string());
            continue;
        }
        let map = scoop_toolchain::profile_map_path(node.artifact_locator());
        if map.try_exists().map_err(output_error)? {
            output.maps.push(map);
        } else {
            output.missing_maps.push(map);
        }
    }
    Ok(output)
}
