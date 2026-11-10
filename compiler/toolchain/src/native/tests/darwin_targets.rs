use super::*;
use object::{Architecture, Object};

#[test]
fn shared_native_cache_keeps_darwin_architectures_separate() {
    let directory = tempfile::tempdir().unwrap();
    let manifest = manifest(
        &directory.path().join("cone"),
        "int answer(void); int answer(void) { return 42; }\n",
    );
    let cache = directory.path().join("cache");
    let mut outputs = Vec::new();
    for (triple, architecture) in [
        ("aarch64-apple-darwin", Architecture::Aarch64),
        ("x86_64-apple-darwin", Architecture::X86_64),
    ] {
        let target = ResolvedTargetProfile::resolve(triple).unwrap();
        let prepare = || {
            prepare_native_objects(
                &manifest,
                target.lir_target(),
                || Ok(&target),
                OptimizationMode::Release,
                &crate::development_runtime_root(),
                &cache,
                scoop_protocol::CompilationPurpose::Production,
            )
            .unwrap()
        };
        let cold = prepare();
        let bytes = std::fs::read(&cold.objects()[0]).unwrap();
        let object = object::File::parse(bytes.as_slice()).unwrap();
        assert_eq!(object.architecture(), architecture, "{triple}");
        assert_eq!(prepare(), cold);
        outputs.push(cold);
    }
    assert_ne!(outputs[0].key(), outputs[1].key());
    assert_ne!(outputs[0].objects(), outputs[1].objects());
}
