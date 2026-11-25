use scoop_pm::lockfile::{Lockfile, Purpose};
use scoop_protocol::{CompilationPurpose, package::PackageLockedPurposesV1};
use std::{collections::BTreeMap, path::Path};

use super::*;

mod why;

#[derive(Args)]
pub(super) struct LockedPurposesArgs {
    lockfile: PathBuf,
    #[arg(long)]
    target: String,
}

pub(super) fn locked_purposes(args: LockedPurposesArgs) -> PmResult<()> {
    let target = scoop_toolchain::select_target(&args.target)
        .map_err(|error| PmError::new(error.code(), error.to_string()))?;
    let triple = target.target().contract().canonical_triple();
    let mut purposes = BTreeMap::<_, Vec<_>>::new();
    for resolution in read_lockfile(&args.lockfile)?.resolutions {
        if resolution.target == triple {
            purposes
                .entry(resolution.root)
                .or_default()
                .push(match resolution.purpose {
                    Purpose::Production => CompilationPurpose::Production,
                    Purpose::Test => CompilationPurpose::Test,
                });
        }
    }
    for purposes in purposes.values_mut() {
        purposes.sort_by_key(|purpose| *purpose == CompilationPurpose::Test);
    }
    let result = PackageLockedPurposesV1 {
        version: PACKAGE_PROTOCOL_VERSION,
        purposes,
    };
    print_line(format_args!(
        "{}",
        serde_json::to_string(&result).map_err(|error| {
            PmError::new(scoop_source::DiagnosticCode::ICE2501, error.to_string())
        })?
    ))
}

#[derive(Args)]
pub(super) struct DepsArgs {
    #[command(flatten)]
    members: MembersArgs,
    #[arg(long)]
    lockfile: Option<PathBuf>,
    #[arg(long, default_value = "production")]
    purpose: Purpose,
    #[arg(long)]
    target: Option<String>,
    /// Explain the locked selection and its incoming version/source requirements.
    #[arg(long, value_name = "GROUP:NAME")]
    why: Option<DependencyCoordinateKey>,
    #[arg(long)]
    sysroot: Option<PathBuf>,
    #[arg(long, value_name = "DIRECTORY")]
    cone_path: Vec<PathBuf>,
    #[arg(long)]
    package_cache_dir: Option<PathBuf>,
}

pub(super) fn execute(args: DepsArgs) -> PmResult<()> {
    if let Some(key) = args.why.clone() {
        return why::execute(args, key);
    }
    let cwd = std::env::current_dir()
        .map_err(|error| PmError::new(scoop_source::DiagnosticCode::E2201, error.to_string()))?;
    let workspace = Workspace::discover(&cwd, args.members.input.as_deref())?;
    let path = args
        .lockfile
        .map(|path| cwd.join(path))
        .unwrap_or_else(|| workspace.root.join("Cone.lock"));
    let lock = read_lockfile(&path)?;
    let target = match args.target {
        Some(target) => target,
        None => scoop_toolchain::host_target_triple()
            .map_err(|error| PmError::new(error.code(), error.to_string()))?
            .into(),
    };
    let selection = if args.members.workspace {
        MemberSelection::All
    } else if args.members.package.is_empty() {
        MemberSelection::Default
    } else {
        MemberSelection::Packages(args.members.package)
    };
    let resolutions = workspace
        .select(&selection)?
        .into_iter()
        .map(|member| {
            lock.resolution(&member.relative_path, args.purpose, &target)
                .cloned()
                .ok_or_else(|| {
                    PmError::new(
                        scoop_source::DiagnosticCode::E2111,
                        format!(
                            "no locked resolution for {} / {} / {target}",
                            member.cone.coordinate, args.purpose
                        ),
                    )
                    .with_help(
                        scoop_source::DiagnosticCode::H2101,
                        "resolve dependencies first",
                    )
                })
        })
        .collect::<PmResult<Vec<_>>>()?;
    if args.members.json {
        print_line(format_args!(
            "{}",
            serde_json::to_string_pretty(&Lockfile {
                format: 1,
                resolutions
            })
            .map_err(|error| PmError::new(
                scoop_source::DiagnosticCode::ICE2501,
                error.to_string()
            ))?
        ))?;
    } else {
        for resolution in resolutions {
            print_line(format_args!(
                "{} [{} / {}]",
                resolution.root_coordinate, resolution.purpose, resolution.target
            ))?;
            for node in resolution.nodes {
                print_line(format_args!("  {} ({})", node.coordinate, node.source))?;
                for dependency in node.dependencies {
                    print_line(format_args!("    -> {dependency}"))?;
                }
                for dependency in node.test_dependencies {
                    print_line(format_args!("    -> {dependency} [test]"))?;
                }
            }
        }
    }
    Ok(())
}

fn read_lockfile(path: &Path) -> PmResult<Lockfile> {
    Lockfile::read(path)?.ok_or_else(|| {
        PmError::new(
            scoop_source::DiagnosticCode::E2111,
            format!("{} does not exist", path.display()),
        )
        .with_help(
            scoop_source::DiagnosticCode::H2101,
            "resolve dependencies first",
        )
    })
}
