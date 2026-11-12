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
    pub fn start() -> io::Result<Self> {
        let mut child = Command::new(std::env::current_exe()?)
            .arg("--stdio")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
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

    pub fn notify(&mut self, method: &str, params: Value) -> io::Result<()> {
        Message::Notification(Notification::new(method.to_owned(), params)).write(&mut self.input)
    }

    pub fn request(&mut self, method: &str, params: Value) -> io::Result<Value> {
        self.next_id += 1;
        let id = RequestId::from(self.next_id);
        Message::Request(Request::new(id.clone(), method.to_owned(), params))
            .write(&mut self.input)?;
        loop {
            if let Some(response) = self.receive()? {
                if response.id != id {
                    return Err(io::Error::other(format!(
                        "unexpected response id {}",
                        response.id
                    )));
                }
                return response.response_result.map_err(|error| {
                    io::Error::other(format!("{method}: {} ({})", error.message, error.code))
                });
            }
        }
    }

    pub fn wait_diagnostics(&mut self, uri: &str, version: i32) -> io::Result<()> {
        while !self
            .diagnostics
            .get(uri)
            .is_some_and(|params| params.version == Some(version))
        {
            if self.receive()?.is_some() {
                return Err(io::Error::other(
                    "unexpected response while waiting for diagnostics",
                ));
            }
        }
        Ok(())
    }

    fn receive(&mut self) -> io::Result<Option<Response>> {
        match Message::read(&mut self.output)? {
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
                .write(&mut self.input)?;
                Ok(None)
            }
            None => {
                let _ = self.child.kill();
                let _ = self.child.wait();
                self.finished = true;
                let details = self.stderr_text()?;
                Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    format!("language server closed before completing the query{details}"),
                ))
            }
        }
    }

    pub fn finish(&mut self) -> io::Result<()> {
        self.request("shutdown", Value::Null)?;
        self.notify("exit", Value::Null)?;
        let status = self.child.wait()?;
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
            Err(io::Error::other(format!(
                "language server exited with {status}{details}"
            )))
        }
    }

    fn stderr_text(&mut self) -> io::Result<String> {
        let bytes = self
            .stderr
            .take()
            .expect("stderr is collected once")
            .join()
            .map_err(|_| io::Error::other("server stderr reader stopped"))??;
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
