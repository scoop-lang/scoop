use super::*;

#[cfg(windows)]
mod windows;

#[test]
fn actual_host_invocation_is_transported_to_the_child() {
    let triple = crate::host_target_triple().unwrap();
    let resolved = ResolvedTargetProfile::resolve(triple).unwrap();
    let child = resolved.child_request().unwrap();
    assert!(child.resolved_toolchain().is_some());
    assert_eq!(
        ResolvedTargetProfile::resolve_request(&child).unwrap(),
        resolved
    );
}
