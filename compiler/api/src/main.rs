use clap::{Parser, Subcommand, ValueEnum};
use scoop_api::ApiError;
use scoop_api::query::{Catalog, Selection};
use scoop_source::DiagnosticCode as C;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Clone, Copy, PartialEq, ValueEnum)]
enum Format {
    Text,
    Json,
}

#[derive(Parser)]
#[command(name = "scoop api", about = "Search published Scoop API documentation without building", color = clap::ColorChoice::Never)]
struct Args {
    #[arg(long, global = true)]
    sysroot: Option<PathBuf>,
    /// A concrete documentation site containing api/index.json.
    #[arg(long, global = true)]
    docs: Option<PathBuf>,
    #[arg(long, global = true)]
    target: Option<String>,
    /// Exact library name or full coordinate, including re-export providers.
    #[arg(long, global = true)]
    cone: Option<String>,
    #[arg(long, global = true)]
    receiver: Option<String>,
    #[arg(long, global = true, value_enum, default_value = "text")]
    format: Format,
    #[command(subcommand)]
    command: Query,
}

#[derive(Subcommand)]
enum Query {
    Search {
        query: String,
        #[arg(long, default_value_t = 20)]
        limit: usize,
        #[arg(long, default_value_t = 0)]
        offset: usize,
    },
    Show {
        symbol: String,
    },
}

fn installation_root(args: &Args) -> Result<PathBuf, ApiError> {
    if let Some(path) = &args.sysroot {
        return Ok(path.clone());
    }
    let executable = std::env::current_exe().map_err(|e| ApiError::new(C::E2201, e))?;
    if let Some(commands) = executable
        .parent()
        .filter(|path| path.file_name().is_some_and(|name| name == "commands"))
    {
        return commands
            .parent()
            .map(Path::to_path_buf)
            .ok_or_else(|| ApiError::new(C::E2302, "cannot locate installation root"));
    }
    std::env::var_os("SCOOP_SYSROOT")
        .map(PathBuf::from)
        .ok_or_else(|| {
            ApiError::new(C::E2302, "cannot locate installed documentation").with_help(
                C::H2301,
                "provide --sysroot or --docs, or install scoop-api in <SYSROOT>/commands",
            )
        })
}

fn run(args: Args) -> Result<String, (u8, ApiError)> {
    let (text, exact, offset, limit) = match &args.command {
        Query::Search {
            query,
            offset,
            limit,
        } => (query, false, *offset, *limit),
        Query::Show { symbol } => (symbol, true, 0, usize::MAX),
    };
    if text.trim().is_empty() || limit == 0 {
        return Err((
            2,
            ApiError::new(
                C::E2503,
                "query must be nonempty and limit must be positive",
            ),
        ));
    }
    let target = match (&args.target, &args.docs) {
        (Some(target), _) => Some(target.clone()),
        (None, Some(_)) => None,
        (None, None) => {
            let environment = if cfg!(target_env = "musl") {
                "musl"
            } else {
                ""
            };
            let host =
                scoop_target_data::host(std::env::consts::OS, std::env::consts::ARCH, environment)
                    .ok_or_else(|| {
                        (
                            2,
                            ApiError::new(C::E2301, "unknown host default")
                                .with_help(C::H2001, "provide --target or --docs"),
                        )
                    })?;
            Some(host.default_target.to_owned())
        }
    };
    if let Some(target) = &target
        && scoop_target_data::named_target(target).is_none()
    {
        return Err((
            2,
            ApiError::new(C::E2301, format!("unknown complete target {target:?}")),
        ));
    }
    let root = if let Some(root) = &args.docs {
        root.clone()
    } else {
        installation_root(&args)
            .map_err(|e| (1, e))?
            .join("docs")
            .join(target.as_deref().unwrap())
    };
    let catalog = Catalog::read(&root, target.as_deref()).map_err(|e| (1, e))?;
    let page = catalog.query(Selection {
        text,
        exact,
        cone: args.cone.as_deref(),
        receiver: args.receiver.as_deref(),
        offset,
        limit,
    });
    if args.format == Format::Json {
        serde_json::to_string_pretty(&page)
            .map(|json| json + "\n")
            .map_err(|e| (1, ApiError::new(C::ICE2501, e)))
    } else {
        Ok(scoop_api::render::text(&page))
    }
}

fn report(code: u8, error: &ApiError, json: bool) -> ExitCode {
    if json {
        eprintln!(
            "{}",
            serde_json::json!({"schema":2,"kind":"error","exitCode":code,"code":error.code,"message":error.message,"notes": error.help.iter().map(|(code, message)| serde_json::json!({"code":code,"kind":"help","message":message})).collect::<Vec<_>>()})
        );
    } else {
        eprintln!("error[{}]: {error}", error.code);
    }
    ExitCode::from(code)
}

fn main() -> ExitCode {
    let raw: Vec<_> = std::env::args_os().collect();
    let json = raw.iter().any(|arg| arg == "--format=json")
        || raw
            .windows(2)
            .any(|args| args[0] == "--format" && args[1] == "json");
    let args = match Args::try_parse_from(raw) {
        Ok(args) => args,
        Err(error) if !error.use_stderr() => {
            let _ = error.print();
            return ExitCode::SUCCESS;
        }
        Err(error) => return report(2, &ApiError::new(C::E2001, error), json),
    };
    match run(args) {
        Ok(text) => match std::io::stdout().lock().write_all(text.as_bytes()) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) if error.kind() == std::io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
            Err(error) => report(1, &ApiError::new(C::E2001, error), json),
        },
        Err((code, message)) => report(code, &message, json),
    }
}
