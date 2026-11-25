//! Single-Cone artifact compiler CLI.

use std::path::PathBuf;
use std::process::ExitCode;
use std::{io, io::Write};

use clap::{Parser, Subcommand, ValueEnum};

mod check;
mod child_protocol;
mod dumps;

// Only Rust allocations use this allocator; LLVM owns its native allocations.
#[cfg(windows)]
#[global_allocator]
static ALLOCATOR: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[derive(Parser)]
#[command(name = "scoopc", version = scoop_toolchain::VERSION, about = "The Scoop compiler")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Compile one Cone into a validated `.slib` artifact.
    Build(Box<BuildArgs>),
    /// Check one Cone without producing machine artifacts.
    Check(check::CheckArgs),
    /// Report the exact machine protocol understood by this compiler.
    #[command(name = "__machine-capability", hide = true)]
    MachineCapability,
    /// Compile exactly one request from the versioned machine transport.
    #[command(name = "__child-protocol", hide = true)]
    ChildProtocol {
        version: u32,
        #[arg(long, default_value_t = std::num::NonZeroUsize::MIN)]
        jobs: std::num::NonZeroUsize,
    },
}

#[derive(clap::Args)]
struct BuildArgs {
    /// One `.scoop` file; use `scoop build` for Cone projects.
    input: PathBuf,
    /// Direct dependency artifact; may be repeated.
    #[arg(long = "direct-slib")]
    direct_slibs: Vec<PathBuf>,
    /// Transitive support dependency artifact; may be repeated.
    #[arg(long = "support-slib")]
    support_slibs: Vec<PathBuf>,
    /// Required destination for the validated current-Cone artifact.
    #[arg(long = "out-slib")]
    out_slib: PathBuf,
    /// Target triple; defaults to the supported host.
    #[arg(long)]
    target: Option<String>,
    /// Compiler optimization profile.
    #[arg(long, value_enum, default_value_t = Profile::Debug)]
    profile: Profile,
    /// Native source debug information: none, line-tables or full.
    #[arg(long)]
    debug_info: Option<scoop_lir::DebugInfoLevel>,
    /// Maximum concurrent compiler workers; defaults to the available parallelism.
    #[arg(short = 'j', long)]
    jobs: Option<std::num::NonZeroUsize>,
    /// Core sysroot; defaults to SCOOP_SYSROOT and the development layout.
    #[arg(long)]
    sysroot: Option<PathBuf>,
    /// Native C compiler driver for the selected target.
    #[arg(long)]
    cc: Option<PathBuf>,
    #[arg(long)]
    cxx: Option<PathBuf>,
    /// Target C development sysroot, separate from the Scoop sysroot.
    #[arg(long)]
    native_sysroot: Option<PathBuf>,
    /// Register a host source generator; may be repeated.
    #[arg(long = "generator", value_name = "NAME=PATH")]
    generators: Vec<std::ffi::OsString>,
    /// Emit compiler observations, including generated source.
    #[arg(long, value_enum)]
    emit: Option<Emit>,
    /// Save observations in this directory instead of printing them to stdout.
    #[arg(long, requires = "emit")]
    dump_dir: Option<PathBuf>,
}

#[derive(Clone, Copy, ValueEnum)]
enum Profile {
    Debug,
    Release,
}

impl Profile {
    fn optimization(self) -> scoop_lir::OptimizationMode {
        match self {
            Self::Debug => scoop_lir::OptimizationMode::Debug,
            Self::Release => scoop_lir::OptimizationMode::Release,
        }
    }
}

#[derive(Clone, Copy, ValueEnum)]
enum Emit {
    Ast,
    Generated,
    Hir,
    Mir,
    Optimizations,
    Lir,
    Symbols,
    All,
}

fn main() -> ExitCode {
    run().unwrap_or_else(|error| {
        // stderr may itself be the stream whose write failed.
        let _ = writeln!(
            io::stderr(),
            "error[E2204]: cannot write scoopc output: {error}"
        );
        ExitCode::FAILURE
    })
}

fn run() -> io::Result<ExitCode> {
    let mut stderr = io::stderr();
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(error) => {
            if error.use_stderr() {
                write!(stderr, "error[E2001]: {error}")?;
                return Ok(ExitCode::from(2));
            }
            let mut stdout = io::stdout().lock();
            write!(stdout, "{error}")?;
            stdout.flush()?;
            return Ok(ExitCode::SUCCESS);
        }
    };
    if let Err(error) = scoop_process::initialize() {
        writeln!(
            stderr,
            "error[E2204]: cannot initialize process signal handling: {error}"
        )?;
        return Ok(ExitCode::FAILURE);
    }
    let result = match cli.command {
        Command::Build(args) => build(*args, &mut stderr),
        Command::Check(args) => check::run(args, &mut stderr),
        Command::MachineCapability => write_machine_capability(&mut stderr),
        Command::ChildProtocol { version, jobs } => Ok(child_protocol::run(version, jobs)),
    };
    scoop_process::finish_interruption();
    result
}

fn write_machine_capability(stderr: &mut impl Write) -> io::Result<ExitCode> {
    let capability = match scoop_toolchain::paired_compiler_machine_capability() {
        Ok(capability) => capability,
        Err(error) => {
            writeln!(
                stderr,
                "error[ICE2501]: cannot construct scoopc machine capability: {error}"
            )?;
            return Ok(ExitCode::FAILURE);
        }
    };
    let frame = match scoop_protocol::encode_machine_capability_frame(&capability) {
        Ok(frame) => frame,
        Err(error) => {
            writeln!(
                stderr,
                "error[ICE2501]: cannot encode scoopc machine capability: {error}"
            )?;
            return Ok(ExitCode::FAILURE);
        }
    };
    let mut stdout = io::stdout().lock();
    if let Err(error) = stdout.write_all(&frame).and_then(|()| stdout.flush()) {
        writeln!(
            stderr,
            "error[E2204]: cannot write scoopc machine capability: {error}"
        )?;
        return Ok(ExitCode::FAILURE);
    }
    Ok(ExitCode::SUCCESS)
}

fn build(args: BuildArgs, stderr: &mut impl Write) -> io::Result<ExitCode> {
    let BuildArgs {
        input,
        direct_slibs,
        support_slibs,
        out_slib,
        emit,
        dump_dir,
        target,
        sysroot,
        cc,
        cxx,
        native_sysroot,
        generators,
        profile,
        debug_info,
        jobs,
    } = args;
    let options = scoopc::DirectBuildOptions {
        optimization: profile.optimization(),
        debug_info: debug_info.unwrap_or(match profile {
            Profile::Debug => scoop_lir::DebugInfoLevel::Full,
            Profile::Release => scoop_lir::DebugInfoLevel::None,
        }),
        target,
        sysroot: sysroot.clone(),
        generators,
        c_toolchain: scoop_toolchain::CToolchainOptions {
            sysroot,
            compiler: cc,
            cxx_compiler: cxx,
            native_sysroot,
        },
    };
    let jobs = jobs.unwrap_or_else(|| {
        std::thread::available_parallelism().unwrap_or(std::num::NonZeroUsize::MIN)
    });
    let emit = match emit {
        None => scoopc::StageDumpPolicy::None,
        Some(emit) => scoopc::StageDumpPolicy::Stages(match emit {
            Emit::Ast => scoop_protocol::StageDumpSet::one(scoopc::StageDumpKind::Ast),
            Emit::Generated => scoop_protocol::StageDumpSet::one(scoopc::StageDumpKind::Generated),
            Emit::Hir => scoop_protocol::StageDumpSet::one(scoopc::StageDumpKind::Hir),
            Emit::Mir => scoop_protocol::StageDumpSet::one(scoopc::StageDumpKind::Mir),
            Emit::Optimizations => {
                scoop_protocol::StageDumpSet::one(scoopc::StageDumpKind::Optimizations)
            }
            Emit::Lir => scoop_protocol::StageDumpSet::one(scoopc::StageDumpKind::Lir),
            Emit::Symbols => scoop_protocol::StageDumpSet::one(scoopc::StageDumpKind::Symbols),
            Emit::All => scoop_protocol::StageDumpSet::all(),
        }),
    };
    let request = match scoopc::normalize_direct_build_request(
        input,
        direct_slibs,
        support_slibs,
        out_slib,
        scoopc::DiagnosticOutputPolicy::Human,
        emit,
        options,
    ) {
        Ok(request) => request.with_jobs(jobs),
        Err(error) => {
            writeln!(stderr, "error[{}]: {error}", error.code())?;
            print_notes(stderr, &error.notes(), 1)?;
            return Ok(ExitCode::FAILURE);
        }
    };
    match request.build_and_publish() {
        Ok(success) => {
            if !success.warnings().is_empty() {
                writeln!(stderr, "{}", success.warnings().render_human())?;
            }
            if let Some(directory) = dump_dir {
                if let Err(error) = dumps::write_files(&directory, success.emitted_dumps()) {
                    writeln!(
                        stderr,
                        "error[E2204]: cannot write compiler observations to {}: {error}",
                        directory.display()
                    )?;
                    return Ok(ExitCode::FAILURE);
                }
            } else {
                let mut stdout = io::stdout().lock();
                for dump in success.emitted_dumps() {
                    writeln!(stdout, "{}", dump.text())?;
                }
                stdout.flush()?;
            }
            Ok(ExitCode::SUCCESS)
        }
        Err(error) => {
            if let Some(warnings) = error.warnings().filter(|warnings| !warnings.is_empty()) {
                writeln!(stderr, "{}", warnings.render_human())?;
            }
            writeln!(stderr, "error[{}]: {error}", error.code())?;
            print_notes(stderr, &error.notes(), 1)?;
            Ok(ExitCode::FAILURE)
        }
    }
}

fn print_notes(
    stderr: &mut impl Write,
    notes: &[scoop_source::SourceDiagnosticNote],
    depth: usize,
) -> io::Result<()> {
    for note in notes {
        writeln!(
            stderr,
            "{}{}[{}]: {}",
            "  ".repeat(depth),
            note.code.role().as_str(),
            note.code,
            note.message
        )?;
        print_notes(stderr, &note.notes, depth + 1)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_observations_can_be_printed_or_saved() {
        for directory in [false, true] {
            let mut args = vec![
                "scoopc",
                "build",
                "main.scoop",
                "--out-slib",
                "main.slib",
                "--emit",
                "generated",
            ];
            if directory {
                args.extend(["--dump-dir", "observations"]);
            }
            let Command::Build(args) = Cli::try_parse_from(args).unwrap().command else {
                panic!("expected build command");
            };
            assert!(matches!(args.emit, Some(Emit::Generated)));
            assert_eq!(
                args.dump_dir,
                directory.then(|| PathBuf::from("observations"))
            );
        }
        assert!(
            Cli::try_parse_from([
                "scoopc",
                "build",
                "main.scoop",
                "--out-slib",
                "main.slib",
                "--dump-dir",
                "observations",
            ])
            .is_err()
        );
    }

    #[test]
    fn formal_build_arguments_are_accepted() {
        let cli = Cli::try_parse_from([
            "scoopc",
            "build",
            "main.scoop",
            "--direct-slib",
            "direct.slib",
            "--support-slib",
            "support.slib",
            "--out-slib",
            "current.slib",
            "--emit",
            "all",
            "--target",
            "arm64-apple-darwin",
            "--sysroot",
            "custom-sysroot",
            "--generator",
            "example=tools/first",
            "--generator",
            "second=tools/second",
        ])
        .unwrap();
        let Command::Build(args) = cli.command else {
            panic!("expected build command");
        };
        let BuildArgs {
            input,
            direct_slibs,
            support_slibs,
            out_slib,
            emit,
            target,
            sysroot,
            generators,
            ..
        } = *args;
        assert_eq!(input, PathBuf::from("main.scoop"));
        assert_eq!(direct_slibs, [PathBuf::from("direct.slib")]);
        assert_eq!(support_slibs, [PathBuf::from("support.slib")]);
        assert_eq!(out_slib, PathBuf::from("current.slib"));
        assert!(matches!(emit, Some(Emit::All)));
        assert_eq!(target.as_deref(), Some("arm64-apple-darwin"));
        assert_eq!(sysroot, Some(PathBuf::from("custom-sysroot")));
        assert_eq!(
            generators,
            [
                std::ffi::OsString::from("example=tools/first"),
                std::ffi::OsString::from("second=tools/second")
            ]
        );
    }

    #[test]
    fn executable_runner_flags_are_rejected() {
        assert!(
            Cli::try_parse_from([
                "scoopc",
                "build",
                "main.scoop",
                "-o",
                "out",
                "--out-slib",
                "main.slib",
            ])
            .is_err()
        );
        assert!(
            Cli::try_parse_from([
                "scoopc",
                "build",
                "main.scoop",
                "-L",
                "native",
                "--out-slib",
                "main.slib",
            ])
            .is_err()
        );
    }

    #[test]
    fn machine_capability_is_a_unique_hidden_command() {
        let cli = Cli::try_parse_from(["scoopc", "__machine-capability"]).unwrap();
        assert!(matches!(cli.command, Command::MachineCapability));
        assert!(Cli::try_parse_from(["scoopc", "__machine-capability", "unexpected"]).is_err());
    }

    #[test]
    fn child_protocol_is_a_unique_versioned_hidden_command() {
        let cli = Cli::try_parse_from(["scoopc", "__child-protocol", "1"]).unwrap();
        assert!(matches!(
            cli.command,
            Command::ChildProtocol {
                version: 1,
                jobs: std::num::NonZeroUsize::MIN,
            }
        ));
        assert!(Cli::try_parse_from(["scoopc", "__child-protocol"]).is_err());
        assert!(Cli::try_parse_from(["scoopc", "__child-protocol", "1", "extra"]).is_err());
    }
}
