use std::{collections::BTreeSet, env, fmt::Write, fs, path::PathBuf};

use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Catalog {
    targets: Vec<Target>,
    hosts: Vec<Host>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Target {
    symbol: String,
    canonical: String,
    profile: String,
    version: u32,
    #[serde(default)]
    aliases: Vec<String>,
    #[serde(default)]
    versioned_aliases: Vec<String>,
    os: String,
    arch: String,
    env: String,
    llvm_triple: String,
    llvm_data_layout: String,
    object_target: String,
    backend: String,
    procedure_abi: String,
    scoop_abi: String,
    exception_model: String,
    tuning: String,
    native_environment: String,
    c_bridge_profile: String,
    native_rust_target: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Host {
    os: String,
    arch: String,
    environment: String,
    system: String,
    machines: Vec<String>,
    rust_target: String,
    default_target: String,
    distribution_targets: Vec<String>,
}

fn identifier(value: &str) {
    assert!(
        value.starts_with(|c: char| c.is_ascii_alphabetic())
            && value.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'),
        "invalid implementation identifier {value:?}"
    );
}

fn target_os(value: &str) -> &'static str {
    match value {
        "darwin" => "Darwin",
        "linux" => "Linux",
        "windows" => "Windows",
        _ => panic!("unsupported target OS {value:?}"),
    }
}

fn target_architecture(value: &str) -> &'static str {
    match value {
        "aarch64" => "Aarch64",
        "x86_64" => "X86_64",
        _ => panic!("unsupported target architecture {value:?}"),
    }
}

fn object_target_architecture(value: &str) -> &'static str {
    match value {
        "MachOAarch64" => "aarch64",
        "MachOX86_64" | "ElfX86_64" | "CoffAmd64" => "x86_64",
        _ => panic!("unsupported object target {value:?}"),
    }
}

fn main() {
    println!("cargo:rerun-if-changed=targets.toml");
    let catalog: Catalog = toml::from_str(&fs::read_to_string("targets.toml").unwrap()).unwrap();
    assert!(!catalog.targets.is_empty(), "target catalog is empty");
    let mut names = BTreeSet::new();
    let mut profiles = BTreeSet::new();
    let mut symbols = BTreeSet::new();
    let mut output = String::new();
    let mut source_key = String::new();
    for target in &catalog.targets {
        for name in std::iter::once(&target.canonical).chain(&target.aliases) {
            assert!(names.insert(name), "duplicate target spelling {name}");
        }
        assert!(
            profiles.insert((&target.profile, target.version)),
            "duplicate target profile"
        );
        assert!(symbols.insert(&target.symbol), "duplicate target symbol");
        for name in [
            &target.symbol,
            &target.object_target,
            &target.backend,
            &target.native_environment,
            &target.procedure_abi,
            &target.scoop_abi,
            &target.exception_model,
            &target.tuning,
        ] {
            identifier(name);
        }
        let Target {
            symbol,
            canonical,
            profile,
            version,
            llvm_data_layout,
            object_target,
            backend,
            procedure_abi,
            scoop_abi,
            exception_model,
            ..
        } = target;
        let architecture = target_architecture(&target.arch);
        assert_eq!(
            target.arch,
            object_target_architecture(object_target),
            "target architecture and object machine differ for {canonical}"
        );
        writeln!(
            output,
            "pub const {symbol}_ID: TargetProfileId = target_id({profile:?}, {version});"
        )
        .unwrap();
        writeln!(output, "pub const {symbol}: LirTargetProfile = LirTargetProfile::new(&{symbol}_ID, &crate::layout::describe({canonical:?}, crate::layout::LayoutFacts {{ llvm_data_layout: {llvm_data_layout:?}, object_target: scoop_lir::ObjectTargetProfileV1::{object_target}, architecture: scoop_lir::TargetArchitecture::{architecture} }}, scoop_lir::ProcedureAbi::{procedure_abi}, scoop_lir::ScoopAbiClassifier::{scoop_abi}, scoop_lir::TargetExceptionModelV1::{exception_model}));").unwrap();
        writeln!(output, "pub const {symbol}_LLVM_22_1: ValidatedLirTargetSelection = ValidatedLirTargetSelection::new({symbol}, crate::{backend});").unwrap();
        writeln!(
            source_key,
            "{canonical}\t{profile}\t{version}\t{}\t{}\t{}",
            target.os, target.arch, target.env
        )
        .unwrap();
    }
    writeln!(
        output,
        "pub const SOURCE_CATALOG_KEY: &str = {source_key:?};"
    )
    .unwrap();
    writeln!(
        output,
        "pub const TARGETS: [TargetDefinition; {}] = [",
        catalog.targets.len()
    )
    .unwrap();
    for target in &catalog.targets {
        let Target {
            symbol,
            aliases,
            versioned_aliases,
            os,
            arch,
            env,
            llvm_triple,
            native_environment,
            c_bridge_profile,
            native_rust_target,
            tuning,
            ..
        } = target;
        let target_os = target_os(os);
        let architecture = target_architecture(arch);
        writeln!(output, "TargetDefinition {{ selection: {symbol}_LLVM_22_1, aliases: &{aliases:?}, versioned_aliases: &{versioned_aliases:?}, os: TargetOs::{target_os}, architecture: scoop_lir::TargetArchitecture::{architecture}, env: {env:?}, llvm_triple: {llvm_triple:?}, native_environment: NativeEnvironment::{native_environment}, c_bridge_profile: {c_bridge_profile:?}, native_rust_target: {native_rust_target:?}, tuning: CodegenTuning::{tuning} }},").unwrap();
    }
    output.push_str("];\npub const fn definition(id: TargetProfileId) -> Option<TargetDefinition> { match id {\n");
    for (index, target) in catalog.targets.iter().enumerate() {
        writeln!(output, "{}_ID => Some(TARGETS[{index}]),", target.symbol).unwrap();
    }
    output.push_str("_ => None, } }\n");
    writeln!(output, "pub const HOSTS: &[HostDefinition] = &[").unwrap();
    let mut hosts = BTreeSet::new();
    for host in &catalog.hosts {
        assert!(
            hosts.insert((&host.os, &host.arch, &host.environment)),
            "duplicate host"
        );
        for target in std::iter::once(&host.default_target).chain(&host.distribution_targets) {
            assert!(
                catalog
                    .targets
                    .iter()
                    .any(|entry| &entry.canonical == target),
                "unknown host target {target}"
            );
        }
        let Host {
            os,
            arch,
            environment,
            system,
            machines,
            rust_target,
            default_target,
            distribution_targets,
        } = host;
        writeln!(output, "HostDefinition {{ os: {os:?}, arch: {arch:?}, environment: {environment:?}, system: {system:?}, machines: &{machines:?}, rust_target: {rust_target:?}, default_target: {default_target:?}, distribution_targets: &{distribution_targets:?} }},").unwrap();
    }
    output.push_str("];\n");
    fs::write(
        PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("catalog.rs"),
        output,
    )
    .unwrap();
}
