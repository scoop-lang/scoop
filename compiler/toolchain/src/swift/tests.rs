use super::*;

#[test]
fn directory_selection_observes_nested_additions_but_ignores_exclusions() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir_all(root.join("nested")).unwrap();
    std::fs::create_dir_all(root.join("excluded")).unwrap();
    std::fs::write(root.join("nested/Main.swift"), "first").unwrap();
    let selected = inputs::Selection::read(
        root.to_path_buf(),
        inputs::Rule::Sources {
            include: Vec::new(),
            exclude: vec![PathBuf::from("excluded")],
        },
    )
    .unwrap();
    std::fs::write(root.join("ignored.txt"), "irrelevant").unwrap();
    std::fs::write(root.join("excluded/NotAnInput.swift"), "irrelevant").unwrap();
    assert!(selected.is_current().unwrap());
    std::fs::create_dir_all(root.join("nested/deeper")).unwrap();
    assert!(selected.is_current().unwrap());
    std::fs::write(root.join("nested/deeper/New.swift"), "selected").unwrap();
    assert!(!selected.is_current().unwrap());
}

#[cfg(target_os = "macos")]
#[test]
fn swift_product_cache_preserves_link_results_for_resource_changes() {
    let temp = tempfile::tempdir().unwrap();
    let package = temp.path().join("package");
    let root = package.as_path();
    let support = temp.path().join("support");
    std::fs::create_dir_all(support.join("Sources/Support")).unwrap();
    std::fs::create_dir_all(support.join("Sources/CHelper/include")).unwrap();
    std::fs::write(
        support.join("Package.swift"),
        r#"// swift-tools-version: 6.3
import PackageDescription
let package = Package(name: "Support", products: [.library(name: "Support", targets: ["Support"])],
    targets: [.target(name: "Support", dependencies: ["CHelper"]), .target(name: "CHelper")])
"#,
    )
    .unwrap();
    std::fs::write(
        support.join("Sources/Support/Support.swift"),
        "import CHelper\npublic func support() -> Int32 { helper() }\n",
    )
    .unwrap();
    std::fs::write(
        support.join("Sources/CHelper/helper.c"),
        "#include <CHelper.h>\nint helper(void) { return EXTRA; }\n",
    )
    .unwrap();
    let header = support.join("Sources/CHelper/include/CHelper.h");
    std::fs::write(&header, "#define EXTRA 1\nint helper(void);\n").unwrap();
    std::fs::create_dir_all(root.join("Sources/Probe/Resources")).unwrap();
    std::fs::write(
        root.join("Package.swift"),
        r#"// swift-tools-version: 6.3
import PackageDescription
let package = Package(name: "Probe", platforms: [.macOS(.v14)],
    products: [.library(name: "Probe", type: .static, targets: ["Probe"])],
    dependencies: [.package(path: "../support")],
    targets: [.target(name: "Probe", dependencies: [.product(name: "Support", package: "support")], resources: [.process("Resources")],
        linkerSettings: [.linkedLibrary("z"), .linkedFramework("Foundation")])])
"#,
    )
    .unwrap();
    std::fs::write(
        root.join("Sources/Probe/Probe.swift"),
        r#"import Foundation
import Support
@c(swift_probe)
public func probe(_ value: Int32) -> Int32 { Int32(String(value).utf8.count) + support() }
public func resource() -> URL { Bundle.module.bundleURL }
"#,
    )
    .unwrap();
    let resource = root.join("Sources/Probe/Resources/value.txt");
    std::fs::write(&resource, "first").unwrap();
    let target = crate::ResolvedTargetProfile::resolve_host().unwrap();
    let manifest = |framework: bool| {
        crate::parse_cone_manifest(&format!(
        "schema = 1\n[cone]\ngroup = 'swift.test'\nname = 'client'\nversion = '0.1.0'\nkind = 'library'\n[[native.swift]]\npackage = '.'\nproduct = 'Probe'\nlinkage = 'static'\n{}",
        if framework { "format = 'framework'\nbundle_id = 'dev.scoop.probe'\n" } else { "" })).unwrap()
    };
    let request = |framework: bool| {
        let manifest = manifest(framework);
        let recipe = manifest
            .semantic()
            .native()
            .swift_libraries(target.id())
            .next()
            .unwrap();
        SwiftBuildRequest::new(
            root,
            recipe,
            target.child_request().unwrap(),
            OptimizationMode::Release,
            SwiftBuildOptions {
                offline: true,
                locked: true,
                ..Default::default()
            },
            false,
            &root.join("cache"),
        )
        .unwrap()
    };
    let build = |request: &SwiftBuildRequest| match request.prepare(true).unwrap() {
        PreparedSwiftBuild::Pending(work) => {
            work.build(&target, NonZeroUsize::new(2).unwrap()).unwrap()
        }
        PreparedSwiftBuild::Complete(product) => *product,
        PreparedSwiftBuild::WaitingForLock => panic!("blocking preparation cannot wait"),
    };
    let request = request(false);
    let first = build(&request);
    assert!(first.output.binary.is_file());
    assert_eq!(first.output.resources.len(), 1);
    assert!(first.output.settings.libraries.contains("z"));
    assert!(first.output.settings.frameworks.contains("Foundation"));
    assert!(matches!(
        request.prepare(false).unwrap(),
        PreparedSwiftBuild::Complete(_)
    ));
    std::fs::write(resource, "changed resource only").unwrap();
    let updated = build(&request);
    assert_eq!(first.link_key, updated.link_key);
    assert_eq!(first.output.binary, updated.output.binary);
    assert_ne!(first.deployment_key, updated.deployment_key);
    let framework_manifest = manifest(true);
    let framework_request = SwiftBuildRequest::new(
        root,
        framework_manifest
            .semantic()
            .native()
            .swift_libraries(target.id())
            .next()
            .unwrap(),
        target.child_request().unwrap(),
        OptimizationMode::Release,
        SwiftBuildOptions {
            offline: true,
            locked: true,
            ..Default::default()
        },
        false,
        &root.join("cache"),
    )
    .unwrap();
    let framework = build(&framework_request);
    assert!(
        framework
            .output
            .library
            .join("Resources/Info.plist")
            .is_file()
    );
    assert!(
        framework
            .output
            .library
            .join("Modules/Probe.swiftmodule")
            .is_dir()
    );
    assert!(matches!(
        framework_request.prepare(false).unwrap(),
        PreparedSwiftBuild::Complete(_)
    ));
    std::fs::write(header, "#define EXTRA 2026\nint helper(void);\n").unwrap();
    let header_changed = build(&request);
    assert_ne!(header_changed.link_key, updated.link_key);
    // An actual selected-file change invalidates production even if its inactive
    // branch would not change emitted code. There is no Swift semantic comparer.
    std::fs::write(
        root.join("Sources/Probe/New.swift"),
        "public let another = 42\n",
    )
    .unwrap();
    let PreparedSwiftBuild::Pending(pending) = request.prepare(false).unwrap() else {
        panic!("new source must invalidate");
    };
    assert!(matches!(
        request.prepare(false).unwrap(),
        PreparedSwiftBuild::WaitingForLock
    ));
    let changed = pending
        .build(&target, NonZeroUsize::new(2).unwrap())
        .unwrap();
    assert_ne!(changed.link_key, first.link_key);
    std::fs::write(root.join("Sources/Probe/New.swift"), "this is not Swift\n").unwrap();
    let PreparedSwiftBuild::Pending(pending) = request.prepare(false).unwrap() else {
        panic!("changed source must invalidate");
    };
    assert!(
        pending
            .build(&target, NonZeroUsize::new(2).unwrap())
            .is_err()
    );
    assert!(matches!(
        request.prepare(false).unwrap(),
        PreparedSwiftBuild::Pending(_)
    ));
}
