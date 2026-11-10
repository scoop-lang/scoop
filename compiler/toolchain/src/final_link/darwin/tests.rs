use super::*;
use crate::ResolvedTargetProfile;

#[test]
fn explicit_lld_path_is_selected_and_missing_tools_are_reported() {
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
    let cold = target.final_link_with_cxx(&options, false).unwrap();
    assert_eq!(cold.linker_driver(), linker);
    let missing = directory.path().join("missing linker");
    let failure = target
        .final_link_with_cxx(
            &FinalLinkOptions {
                linker: Some(missing.clone()),
                ..Default::default()
            },
            false,
        )
        .unwrap_err();
    assert!(failure.to_string().contains("Mach-O LLD not found"));
    assert!(failure.to_string().contains(missing.to_str().unwrap()));
}
