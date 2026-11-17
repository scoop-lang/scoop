use super::*;
use scoop_process::CommandExt;
use std::process::Command;

#[test]
fn shared_source_requires_a_resolved_choice_and_reuses_it_without_updates() {
    let temporary = tempfile::tempdir().unwrap();
    let dependency = temporary.path().join("dependency");
    std::fs::create_dir_all(dependency.join("Sources/Support")).unwrap();
    std::fs::write(dependency.join("Package.swift"), "// swift-tools-version: 6.3\nimport PackageDescription\nlet package = Package(name: \"Support\", products: [.library(name: \"Support\", targets: [\"Support\"])], targets: [.target(name: \"Support\")])\n").unwrap();
    std::fs::write(
        dependency.join("Sources/Support/Value.swift"),
        "public func value() -> Int32 { 42 }\n",
    )
    .unwrap();
    let git = |args: &[&str]| {
        let output = Command::new("git")
            .args([
                "-c",
                "user.name=Swift Test",
                "-c",
                "user.email=swift-test@example.invalid",
                "-c",
                "commit.gpgsign=false",
            ])
            .args(args)
            .current_dir(&dependency)
            .scoop_output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    };
    git(&["init", "-q"]);
    git(&["add", "."]);
    git(&["commit", "-qm", "initial"]);
    git(&["tag", "1.0.0"]);
    let package = temporary.path().join("package");
    std::fs::create_dir_all(package.join("Sources/Probe")).unwrap();
    std::fs::write(package.join("Package.swift"), format!("// swift-tools-version: 6.3\nimport PackageDescription\nlet package = Package(name: \"Probe\", products: [.library(name: \"Probe\", type: .static, targets: [\"Probe\"])], dependencies: [.package(url: \"{}\", exact: \"1.0.0\")], targets: [.target(name: \"Probe\", dependencies: [.product(name: \"Support\", package: \"dependency\")])])\n", dependency.display())).unwrap();
    std::fs::write(
        package.join("Sources/Probe/Probe.swift"),
        "import Support\n@c(swift_probe) public func probe() -> Int32 { value() }\n",
    )
    .unwrap();
    let manifest = crate::parse_cone_manifest("schema = 1\n[cone]\ngroup = 'swift.test'\nname = 'resolved'\nversion = '0.1.0'\nkind = 'library'\n[[native.swift]]\npackage = '.'\nproduct = 'Probe'\nlinkage = 'static'\n").unwrap();
    let target = ResolvedTargetProfile::resolve_host().unwrap();
    let request = |shared, cache: &str| {
        SwiftBuildRequest::new(
            &package,
            manifest
                .semantic()
                .native()
                .swift_libraries(target.id())
                .next()
                .unwrap(),
            target.child_request().unwrap(),
            OptimizationMode::Release,
            SwiftBuildOptions::default(),
            shared,
            &temporary.path().join(cache),
        )
        .unwrap()
    };
    let build = |request: &SwiftBuildRequest| {
        let PreparedSwiftBuild::Pending(work) = request.prepare(true).unwrap() else {
            panic!("expected uncached producer");
        };
        work.build(&target, NonZeroUsize::new(2).unwrap())
    };
    assert!(build(&request(true, "shared-missing")).is_err());
    assert!(!package.join("Package.resolved").exists());
    build(&request(false, "local")).unwrap();
    let resolved = std::fs::read(package.join("Package.resolved")).unwrap();
    let shared = request(true, "shared");
    let built = build(&shared).unwrap();
    assert!(built.output.binary.is_file());
    assert_eq!(
        std::fs::read(package.join("Package.resolved")).unwrap(),
        resolved
    );
    git(&["tag", "1.1.0"]);
    assert!(matches!(
        shared.prepare(false).unwrap(),
        PreparedSwiftBuild::Complete(_)
    ));
    let original = std::fs::read_to_string(package.join("Package.swift")).unwrap();
    std::fs::write(
        package.join("Package.swift"),
        original.replace("1.0.0", "1.1.0"),
    )
    .unwrap();
    assert!(build(&shared).is_err());
    assert_eq!(
        std::fs::read(package.join("Package.resolved")).unwrap(),
        resolved
    );
}
