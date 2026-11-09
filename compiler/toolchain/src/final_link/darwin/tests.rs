use super::*;
use crate::ResolvedTargetProfile;

#[test]
fn explicit_lld_path_is_probed_and_reused_without_republishing() {
    let target = ResolvedTargetProfile::resolve(crate::host_target_triple().unwrap()).unwrap();
    let default = target
        .final_link_with(&FinalLinkOptions::default())
        .unwrap();
    let directory = tempfile::Builder::new()
        .prefix("scoop linker , ")
        .tempdir()
        .unwrap();
    // The explicit flavor must also work when argv[0] is not a standard LLD alias.
    let linker = directory.path().join("selected linker");
    std::os::unix::fs::symlink(default.linker_driver(), &linker).unwrap();
    let options = FinalLinkOptions {
        linker: Some(linker.clone()),
        ..Default::default()
    };
    let cache = directory.path().join("cache");
    let cold = target
        .final_link_with_cxx_cached(&options, false, &cache)
        .unwrap();
    assert_eq!(cold.linker_driver(), linker);
    assert!(cold.input_paths().unwrap().contains(&linker));
    let records = || {
        let mut records: Vec<_> =
            std::fs::read_dir(cache.join(super::super::cache::CACHE_DIRECTORY))
                .unwrap()
                .map(|entry| entry.unwrap().path())
                .filter(|path| {
                    path.extension()
                        .is_some_and(|extension| extension == "json")
                })
                .map(|path| {
                    let modified = path.metadata().unwrap().modified().unwrap();
                    (path, modified)
                })
                .collect();
        records.sort();
        records
    };
    let before = records();
    assert_eq!(before.len(), 1);
    assert_eq!(
        cold,
        target
            .final_link_with_cxx_cached(&options, false, &cache)
            .unwrap()
    );
    assert_eq!(before, records());

    let missing = directory.path().join("missing linker");
    let failure = target
        .final_link_with_cxx_cached(
            &FinalLinkOptions {
                linker: Some(missing.clone()),
                ..Default::default()
            },
            false,
            &cache,
        )
        .unwrap_err();
    assert!(failure.to_string().contains("Mach-O LLD not found"));
    assert!(failure.to_string().contains(missing.to_str().unwrap()));
    assert_eq!(before, records());
}
