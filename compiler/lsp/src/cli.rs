mod client;
mod query;
mod render;

use std::{io::Write, path::PathBuf};

use clap::{Args, Parser, Subcommand, ValueEnum};
use serde_json::json;

#[derive(Parser)]
#[command(name = "scoop-lsp", version = scoop_toolchain::VERSION, about = "Scoop language server and one-shot source queries", args_conflicts_with_subcommands = true)]
struct Cli {
    /// Run the Language Server Protocol over stdin/stdout (the default).
    #[arg(long)]
    stdio: bool,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Query source using the same workspace and frontend as the language server.
    Query(Box<Query>),
}

#[derive(Args)]
struct Query {
    /// Project directory or Cone.toml; file arguments remain relative to the current directory.
    #[arg(long, global = true, default_value = ".")]
    project: PathBuf,
    /// Installed target libraries and source attachments.
    #[arg(long, global = true)]
    sysroot: Option<PathBuf>,
    /// Target triple used to select conditional sources and dependencies.
    #[arg(long, global = true)]
    target: Option<String>,
    #[arg(long, global = true, value_enum, default_value = "production")]
    purpose: Purpose,
    /// Companion PM executable; defaults to scoop-pm beside this server.
    #[arg(long, global = true)]
    package_manager: Option<PathBuf>,
    /// Existing package source cache.
    #[arg(long, global = true)]
    package_cache_dir: Option<PathBuf>,
    /// Use this project's lockfile path.
    #[arg(long, global = true)]
    lockfile: Option<PathBuf>,
    /// Additional artifact search root; repeat for multiple roots.
    #[arg(long, global = true)]
    cone_path: Vec<PathBuf>,
    /// Unit of the one-based input column and all JSON LSP character offsets.
    #[arg(long, global = true, value_enum, default_value = "utf-32")]
    position_encoding: Encoding,
    #[arg(long, global = true, value_enum, default_value = "text")]
    format: Format,
    #[command(subcommand)]
    kind: QueryKind,
}

#[derive(Clone, Copy, ValueEnum)]
enum Purpose {
    Production,
    Test,
}

#[derive(Clone, Copy, ValueEnum)]
enum Encoding {
    #[value(name = "utf-8")]
    Utf8,
    #[value(name = "utf-16")]
    Utf16,
    #[value(name = "utf-32")]
    Utf32,
}

impl Encoding {
    fn name(self) -> &'static str {
        match self {
            Self::Utf8 => "utf-8",
            Self::Utf16 => "utf-16",
            Self::Utf32 => "utf-32",
        }
    }
}

#[derive(Clone, Copy, ValueEnum)]
enum Format {
    Text,
    Json,
}

#[derive(Subcommand)]
enum QueryKind {
    /// Type, signature and documentation at a source position.
    Hover(Position),
    /// Signatures applicable to the enclosing call.
    Signature(Position),
    /// Members and names available at this position.
    Completion(Position),
    /// Declaration selected by this reference.
    Definition(Position),
    /// Declaration of the value's type.
    TypeDefinition(Position),
    /// Implementations of an interface or member.
    Implementation(Position),
    /// References, including the declaration.
    References(Position),
    /// Declarations in this document.
    Symbols(Document),
    /// Source declarations in the selected workspace.
    WorkspaceSymbols { name: String },
    /// Syntax, semantic and project diagnostics for this document.
    Diagnostics(Document),
    /// Actual server capabilities, without loading a project.
    Capabilities,
}

#[derive(Args)]
struct Document {
    file: PathBuf,
    /// Read the complete document buffer from stdin without writing it to disk.
    #[arg(long)]
    stdin: bool,
}

#[derive(Args)]
struct Position {
    #[command(flatten)]
    document: Document,
    #[arg(long, value_parser = clap::value_parser!(u32).range(1..))]
    line: u32,
    #[arg(long, value_parser = clap::value_parser!(u32).range(1..))]
    column: u32,
}

pub fn run() -> i32 {
    let args: Vec<_> = std::env::args_os().collect();
    let cli = match Cli::try_parse_from(&args) {
        Ok(cli) => cli,
        Err(error) => {
            if !error.use_stderr() {
                let _ = error.print();
                return 0;
            }
            let json = args.iter().any(|arg| arg == "--format=json")
                || args
                    .windows(2)
                    .any(|args| args[0] == "--format" && args[1] == "json");
            return failure(
                if json { Format::Json } else { Format::Text },
                "usage",
                error,
                2,
            );
        }
    };
    match cli.command {
        None => match scoop_lsp::run_stdio() {
            Ok(code) => code,
            Err(error) => failure(Format::Text, "server", error, 1),
        },
        Some(Command::Query(query)) => match query::execute(&query) {
            Ok(result) => {
                let output = match query.format {
                    Format::Text => render::text(&result),
                    Format::Json => serde_json::to_string_pretty(&result).map(|s| s + "\n"),
                };
                match output {
                    Ok(text) => match std::io::stdout().lock().write_all(text.as_bytes()) {
                        Ok(()) => 0,
                        Err(error) => failure(query.format, "output", error, 1),
                    },
                    Err(error) => failure(query.format, "output", error, 1),
                }
            }
            Err(error) => failure(query.format, "query", error, 1),
        },
    }
}

fn failure(format: Format, kind: &str, error: impl std::fmt::Display, code: i32) -> i32 {
    match format {
        Format::Text => eprintln!("scoop-lsp: {error}"),
        Format::Json => eprintln!(
            "{}",
            json!({"schema": 1, "kind": "error", "error": {"kind": kind, "message": error.to_string()}})
        ),
    }
    code
}
