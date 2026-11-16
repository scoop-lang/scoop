use super::error::{QueryError, QueryResult};
use scoop_source::DiagnosticCode as C;
use std::{
    collections::BTreeMap,
    io::{self, BufReader, Read},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
};

use lsp_server::{ErrorCode, Message, Notification, Request, RequestId, Response};
use lsp_types::{LogMessageParams, PublishDiagnosticsParams};
use serde_json::Value;

pub(super) struct Client {
    child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
    next_id: i32,
    finished: bool,
    stderr: Option<std::thread::JoinHandle<io::Result<Vec<u8>>>>,
    pub diagnostics: BTreeMap<String, PublishDiagnosticsParams>,
    pub messages: Vec<LogMessageParams>,
}

impl Client {
    pub fn start() -> QueryResult<Self> {
        let mut child = Command::new(std::env::current_exe()?)
            .arg("--stdio")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| QueryError::new(C::E2202, error))?;
        let mut stderr = child.stderr.take().expect("piped stderr");
        let stderr = std::thread::spawn(move || {
            let mut bytes = Vec::new();
            stderr.read_to_end(&mut bytes)?;
            Ok(bytes)
        });
        Ok(Self {
            input: child.stdin.take().expect("piped stdin"),
            output: BufReader::new(child.stdout.take().expect("piped stdout")),
            child,
            next_id: 0,
            finished: false,
            stderr: Some(stderr),
            diagnostics: BTreeMap::new(),
            messages: Vec::new(),
        })
    }

    pub fn notify(&mut self, method: &str, params: Value) -> QueryResult<()> {
        Message::Notification(Notification::new(method.to_owned(), params))
            .write(&mut self.input)
            .map_err(QueryError::transport)
    }

    pub fn request(&mut self, method: &str, params: Value) -> QueryResult<Value> {
        self.next_id += 1;
        let id = RequestId::from(self.next_id);
        Message::Request(Request::new(id.clone(), method.to_owned(), params))
            .write(&mut self.input)
            .map_err(QueryError::transport)?;
        loop {
            if let Some(response) = self.receive()? {
                if response.id != id {
                    return Err(QueryError::new(
                        C::ICE2501,
                        format!("unexpected response id {}", response.id),
                    ));
                }
                return response
                    .response_result
                    .map_err(|error| QueryError::response(method, error));
            }
        }
    }

    pub fn wait_diagnostics(&mut self, uri: &str, version: i32) -> QueryResult<()> {
        while !self
            .diagnostics
            .get(uri)
            .is_some_and(|params| params.version == Some(version))
        {
            if self.receive()?.is_some() {
                return Err(QueryError::new(
                    C::ICE2501,
                    "unexpected response while waiting for diagnostics",
                ));
            }
        }
        Ok(())
    }

    fn receive(&mut self) -> QueryResult<Option<Response>> {
        match Message::read(&mut self.output).map_err(QueryError::transport)? {
            Some(Message::Response(response)) => Ok(Some(response)),
            Some(Message::Notification(notification)) => {
                match notification.method.as_str() {
                    "textDocument/publishDiagnostics" => {
                        let params: PublishDiagnosticsParams =
                            serde_json::from_value(notification.params)?;
                        self.diagnostics
                            .insert(params.uri.as_str().to_owned(), params);
                    }
                    "window/logMessage" | "window/showMessage" => {
                        self.messages
                            .push(serde_json::from_value(notification.params)?);
                    }
                    _ => {}
                }
                Ok(None)
            }
            Some(Message::Request(request)) => {
                Message::Response(Response::new_err(
                    request.id,
                    ErrorCode::MethodNotFound as i32,
                    format!("CLI client does not support {}", request.method),
                ))
                .write(&mut self.input)
                .map_err(QueryError::transport)?;
                Ok(None)
            }
            None => {
                let _ = self.child.kill();
                let _ = self.child.wait();
                self.finished = true;
                let details = self.stderr_text()?;
                Err(QueryError::new(
                    C::E2204,
                    format!("language server closed before completing the query{details}"),
                ))
            }
        }
    }

    pub fn finish(&mut self) -> QueryResult<()> {
        self.request("shutdown", Value::Null)?;
        self.notify("exit", Value::Null)?;
        let status = self.child.wait().map_err(QueryError::transport)?;
        self.finished = true;
        let details = self.stderr_text()?;
        if status.success() {
            if !details.is_empty() {
                self.messages.push(LogMessageParams {
                    typ: lsp_types::MessageType::WARNING,
                    message: format!("server stderr{details}"),
                });
            }
            Ok(())
        } else {
            Err(QueryError::new(
                C::E2203,
                format!("language server exited with {status}{details}"),
            ))
        }
    }

    fn stderr_text(&mut self) -> QueryResult<String> {
        let bytes = self
            .stderr
            .take()
            .expect("stderr is collected once")
            .join()
            .map_err(|_| QueryError::new(C::ICE2502, "server stderr reader stopped"))??;
        let text = String::from_utf8_lossy(&bytes);
        Ok(if text.trim().is_empty() {
            String::new()
        } else {
            format!(": {}", text.trim())
        })
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        if !self.finished {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
        if let Some(reader) = self.stderr.take() {
            let _ = reader.join();
        }
    }
}
