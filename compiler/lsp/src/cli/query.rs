use std::{
    io::{self, Read},
    path::{Path, PathBuf},
};

use lsp_types::{DiagnosticSeverity, LogMessageParams, PublishDiagnosticsParams};
use serde::Serialize;
use serde_json::{Value, json};

use super::{Document, Purpose, Query, QueryKind, client::Client};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Output {
    schema: u32,
    kind: &'static str,
    pub query: &'static str,
    pub status: &'static str,
    position_encoding: &'static str,
    range_base: u32,
    pub result: Value,
    pub diagnostics: Vec<PublishDiagnosticsParams>,
    pub messages: Vec<LogMessageParams>,
}

pub(super) fn execute(query: &Query) -> io::Result<Output> {
    let capabilities = matches!(query.kind, QueryKind::Capabilities);
    let root = if capabilities {
        None
    } else {
        Some(project_root(&query.project)?)
    };
    let document = query
        .kind
        .document()
        .map(|document| {
            read_document(document).map_err(|error| {
                io::Error::new(
                    error.kind(),
                    format!("{}: {error}", document.file.display()),
                )
            })
        })
        .transpose()?;
    let mut client = Client::start()?;
    let initialized = client.request("initialize", json!({
        "processId": std::process::id(),
        "capabilities": {
            "general": {"positionEncodings": [query.position_encoding.name()]},
            "textDocument": {"documentSymbol": {"hierarchicalDocumentSymbolSupport": true}}
        },
        "workspaceFolders": root.as_ref().map(|path| file_uri(path).map(|uri| vec![json!({"uri": uri, "name": path.display().to_string()})])).transpose()?,
        "initializationOptions": {
            "target": query.target,
            "sysroot": absolute_option(&query.sysroot)?,
            "purpose": match query.purpose { Purpose::Production => "production", Purpose::Test => "test" },
            "packageManager": absolute_option(&query.package_manager)?,
            "packageCacheDir": absolute_option(&query.package_cache_dir)?,
            "lockfile": absolute_option(&query.lockfile)?,
            "artifactSearchRoots": query.cone_path.iter().map(std::path::absolute).collect::<io::Result<Vec<_>>>()?,
        }
    }))?;
    let negotiated = initialized["capabilities"]["positionEncoding"]
        .as_str()
        .unwrap_or("utf-16");
    if negotiated != query.position_encoding.name() {
        return Err(io::Error::other(format!(
            "server selected {negotiated} instead of {}",
            query.position_encoding.name()
        )));
    }
    client.notify("initialized", json!({}))?;
    let mut result = if capabilities {
        initialized
    } else if let Some((uri, text)) = &document {
        client.notify(
            "textDocument/didOpen",
            json!({"textDocument": {
                "uri": uri, "languageId": "scoop", "version": 1, "text": text,
            }}),
        )?;
        let (method, params) = query.kind.request(uri);
        let value = client.request(method, params)?;
        // Manifests publish project diagnostics during refresh rather than through the worker.
        if !query
            .kind
            .document()
            .is_some_and(|d| d.file.file_name().is_some_and(|name| name == "Cone.toml"))
        {
            client.wait_diagnostics(uri, 1)?;
        }
        value
    } else if let QueryKind::WorkspaceSymbols { name } = &query.kind {
        client.request("workspace/symbol", json!({"query": name}))?
    } else {
        return Err(io::Error::other("query requires a document"));
    };
    // Read notifications preceding the shutdown response before publishing the complete result.
    client.finish()?;
    let diagnostics: Vec<_> = client.diagnostics.values().cloned().collect();
    if matches!(query.kind, QueryKind::Diagnostics(_)) {
        result = serde_json::to_value(&diagnostics)?;
    }
    let partial = diagnostics.iter().any(|params| {
        params
            .diagnostics
            .iter()
            .any(|d| d.severity == Some(DiagnosticSeverity::ERROR))
    }) || !client.messages.is_empty();
    let empty = if matches!(query.kind, QueryKind::Diagnostics(_)) {
        diagnostics
            .iter()
            .all(|params| params.diagnostics.is_empty())
    } else {
        result.is_null()
            || result.as_array().is_some_and(Vec::is_empty)
            || result
                .get("items")
                .and_then(Value::as_array)
                .is_some_and(Vec::is_empty)
            || result
                .get("signatures")
                .and_then(Value::as_array)
                .is_some_and(Vec::is_empty)
    };
    Ok(Output {
        schema: 1,
        kind: "query",
        query: query.kind.name(),
        status: if partial {
            "partial"
        } else if empty {
            "empty"
        } else {
            "ok"
        },
        position_encoding: query.position_encoding.name(),
        range_base: 0,
        result,
        diagnostics,
        messages: std::mem::take(&mut client.messages),
    })
}

fn project_root(path: &Path) -> io::Result<PathBuf> {
    let path = path
        .canonicalize()
        .map_err(|error| io::Error::new(error.kind(), format!("{}: {error}", path.display())))?;
    let root = if path.file_name().is_some_and(|name| name == "Cone.toml") {
        path.parent()
            .ok_or_else(|| io::Error::other("Cone.toml has no parent directory"))?
            .to_owned()
    } else {
        path
    };
    if !root.join("Cone.toml").is_file() {
        return Err(io::Error::other(format!(
            "{} has no Cone.toml; select a Cone project with --project",
            root.display()
        )));
    }
    Ok(root)
}

fn read_document(document: &Document) -> io::Result<(String, String)> {
    let mut text = String::new();
    let path = if document.stdin {
        io::stdin().read_to_string(&mut text)?;
        // A new buffer may not exist yet; resolve its existing parent in the same way as disk files.
        match document.file.canonicalize() {
            Ok(path) => path,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                let path = std::path::absolute(&document.file)?;
                let parent = path
                    .parent()
                    .ok_or_else(|| io::Error::other("document has no parent directory"))?;
                parent.canonicalize()?.join(
                    path.file_name()
                        .ok_or_else(|| io::Error::other("document requires a file name"))?,
                )
            }
            Err(error) => return Err(error),
        }
    } else {
        let path = document.file.canonicalize()?;
        text = std::fs::read_to_string(&path)?;
        path
    };
    Ok((file_uri(&path)?, text))
}

fn file_uri(path: &Path) -> io::Result<String> {
    url::Url::from_file_path(path)
        .map(String::from)
        .map_err(|()| {
            io::Error::other(format!("cannot represent {} as a file URI", path.display()))
        })
}

fn absolute_option(path: &Option<PathBuf>) -> io::Result<Option<PathBuf>> {
    path.as_ref().map(std::path::absolute).transpose()
}

impl QueryKind {
    fn document(&self) -> Option<&Document> {
        match self {
            Self::Hover(p)
            | Self::Signature(p)
            | Self::Completion(p)
            | Self::Definition(p)
            | Self::TypeDefinition(p)
            | Self::Implementation(p)
            | Self::References(p) => Some(&p.document),
            Self::Symbols(d) | Self::Diagnostics(d) => Some(d),
            Self::WorkspaceSymbols { .. } | Self::Capabilities => None,
        }
    }

    fn name(&self) -> &'static str {
        match self {
            Self::Hover(_) => "hover",
            Self::Signature(_) => "signature",
            Self::Completion(_) => "completion",
            Self::Definition(_) => "definition",
            Self::TypeDefinition(_) => "type-definition",
            Self::Implementation(_) => "implementation",
            Self::References(_) => "references",
            Self::Symbols(_) => "symbols",
            Self::Diagnostics(_) => "diagnostics",
            Self::WorkspaceSymbols { .. } => "workspace-symbols",
            Self::Capabilities => "capabilities",
        }
    }

    fn request(&self, uri: &str) -> (&'static str, Value) {
        let (method, position) = match self {
            Self::Hover(p) => ("textDocument/hover", Some(p)),
            Self::Signature(p) => ("textDocument/signatureHelp", Some(p)),
            Self::Completion(p) => ("textDocument/completion", Some(p)),
            Self::Definition(p) => ("textDocument/definition", Some(p)),
            Self::TypeDefinition(p) => ("textDocument/typeDefinition", Some(p)),
            Self::Implementation(p) => ("textDocument/implementation", Some(p)),
            Self::References(p) => ("textDocument/references", Some(p)),
            Self::Symbols(_) => ("textDocument/documentSymbol", None),
            // A cheap document query orders the response after workspace refresh and didOpen.
            Self::Diagnostics(_) => ("textDocument/documentSymbol", None),
            Self::WorkspaceSymbols { name } => return ("workspace/symbol", json!({"query": name})),
            Self::Capabilities => return ("initialize", Value::Null),
        };
        let mut params = json!({"textDocument": {"uri": uri}});
        if let Some(position) = position {
            params["position"] =
                json!({"line": position.line - 1, "character": position.column - 1});
        }
        if matches!(self, Self::References(_)) {
            params["context"] = json!({"includeDeclaration": true});
        }
        (method, params)
    }
}
