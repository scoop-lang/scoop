use super::*;
use scoop_manifest::{NativeCompileFlag, NativeSourceLanguage};

pub(super) fn configuration(
    manifest: &LoadedConeManifest,
    target: scoop_lir::LirTargetProfile,
    optimization: OptimizationMode,
    sysroot: &Path,
) -> Result<Digest256, ToolchainError> {
    let config = manifest.parsed().semantic().native();
    let target_facts = crate::manifest::source_target(target.id())?;
    let mut sources: Vec<_> = config
        .sources()
        .iter()
        .filter(|s| s.predicate().matches(target.id()))
        .map(|s| {
            config
                .source_language(s.path(), target_facts)
                .map(|language| (s.path().as_str(), language.input_name()))
                .map_err(error)
        })
        .collect::<Result<_, _>>()?;
    sources.sort();
    let flags = |language: NativeSourceLanguage| -> &[NativeCompileFlag] {
        if sources.iter().any(|(_, l)| *l == language.input_name()) {
            config.flags(language)
        } else {
            &[]
        }
    };
    let libraries = config
        .library_requirements(target.id())
        .map_err(error)?
        .iter()
        .map(scoop_wire::encode)
        .collect::<Result<Vec<_>, _>>()
        .map_err(error)?;
    let include: Vec<_> = config
        .includes(target.id())
        .filter(|_| !sources.is_empty())
        .collect();
    fingerprint(&(
        HostPathCarrier::from_path(manifest.real_root()).map_err(error)?,
        &sources,
        include,
        flags(NativeSourceLanguage::C),
        flags(NativeSourceLanguage::Cxx),
        flags(NativeSourceLanguage::ObjectiveC),
        config.cxx(target.id()),
        libraries,
        scoop_wire::encode(&optimization).map_err(error)?,
        (!sources.is_empty())
            .then(|| HostPathCarrier::from_path(sysroot))
            .transpose()
            .map_err(error)?,
        target.contract().canonical_triple(),
    ))
}
