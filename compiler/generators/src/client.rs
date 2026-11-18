use lsp_server::{Message, Request, RequestId, Response};
use scoop_generator_protocol::{
    Generated, GeneratedBody, GeneratorDiagnostic, GeneratorNote, GeneratorSeverity, MethodId,
    PROTOCOL_VERSION, SourceLocation,
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::io::{self, BufReader, BufWriter, Stdin, Stdout};

use crate::model::Members;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Batch {
    protocol_version: u32,
    generator: String,
    core_cone: String,
    tasks: Vec<Task>,
}

#[derive(Deserialize)]
struct Task {
    owner: Value,
    methods: Vec<Method>,
}

#[derive(Deserialize)]
pub struct Requirement {
    pub interface: Value,
}

#[derive(Deserialize)]
pub struct Method {
    pub method: MethodId,
    pub requirements: Vec<Requirement>,
    pub signature: Signature,
    origin: SourceLocation,
}

#[derive(Deserialize)]
pub struct Signature {
    pub name: String,
    pub parameters: Vec<Parameter>,
}

#[derive(Deserialize)]
pub struct Parameter {
    pub name: String,
}

pub struct Error {
    message: String,
    location: Option<Box<SourceLocation>>,
    notes: Vec<GeneratorNote>,
}
pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            location: None,
            notes: vec![],
        }
    }
    pub fn at(location: &SourceLocation, message: impl Into<String>) -> Self {
        Self {
            location: Some(Box::new(location.clone())),
            ..Self::new(message)
        }
    }
    pub fn note(mut self, location: &SourceLocation, message: &str) -> Self {
        self.notes.push(GeneratorNote {
            code: None,
            notes: vec![],
            import_path: vec![],
            message: message.into(),
            location: Some(location.clone()),
        });
        self
    }
    pub fn at_if_missing(mut self, location: &SourceLocation) -> Self {
        self.location
            .get_or_insert_with(|| Box::new(location.clone()));
        self
    }
}
impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        Self::new(error.to_string())
    }
}
impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Self::new(error.to_string())
    }
}

struct Connection {
    input: BufReader<Stdin>,
    output: BufWriter<Stdout>,
    next_id: u64,
    exports: HashMap<(String, Vec<String>), Value>,
}

pub struct Context<'a> {
    connection: &'a mut Connection,
    pub core: &'a str,
    pub owner: &'a Value,
    pub method: &'a Method,
    types: HashMap<Value, Value>,
    members: HashMap<Value, Members>,
    pub imports: Vec<(String, String)>,
}

impl Context<'_> {
    pub fn query(&mut self, query: Value) -> Result<Value> {
        let io = &mut self.connection;
        io.next_id += 1;
        let id = RequestId::from(io.next_id.to_string());
        let params = json!({"context": self.method.method, "query": query});
        Message::Request(Request::new(id.clone(), "compiler/query".into(), params))
            .write(&mut io.output)?;
        let Some(Message::Response(response)) = Message::read(&mut io.input)? else {
            return Err(Error::new("expected a compiler/query response"));
        };
        if response.id != id {
            return Err(Error::new("unexpected compiler/query response id"));
        }
        response.response_result.map_err(|error| {
            let detail = error
                .data
                .as_ref()
                .and_then(|v| v["diagnostics"][0]["message"].as_str())
                .unwrap_or(&error.message);
            Error::new(format!("compiler query failed: {detail}"))
        })
    }

    pub fn describe(&mut self, ty: &Value) -> Result<Value> {
        if let Some(value) = self.types.get(ty) {
            return Ok(value.clone());
        }
        let response = self.query(json!({"kind":"describeTypes", "types":[ty]}))?;
        let value = response["types"][0].clone();
        self.types.insert(ty.clone(), value.clone());
        Ok(value)
    }

    pub fn members(&mut self, ty: &Value) -> Result<Members> {
        if let Some(value) = self.members.get(ty) {
            return Ok(value.clone());
        }
        let response = self.query(json!({"kind":"describeMembers", "types":[ty]}))?;
        let value: Members = serde_json::from_value(response["members"][0].clone())?;
        self.members.insert(ty.clone(), value.clone());
        Ok(value)
    }

    pub fn export(&mut self, provider: &str, path: &[&str]) -> Result<Value> {
        let key = (
            provider.to_owned(),
            path.iter().map(|s| (*s).to_owned()).collect(),
        );
        if let Some(id) = self.connection.exports.get(&key) {
            return Ok(id.clone());
        }
        let response = self.query(
            json!({"kind":"lookupExport","provider":provider,"path":path,"namespace":"type"}),
        )?;
        let [export] = response["exports"]
            .as_array()
            .map(Vec::as_slice)
            .unwrap_or_default()
        else {
            return Err(Error::new(format!(
                "expected one type export for {}",
                path.join(".")
            )));
        };
        let id = export["id"].clone();
        self.connection.exports.insert(key, id.clone());
        Ok(id)
    }

    pub fn core_id(&mut self, name: &str) -> Result<Value> {
        self.export(self.core, &[name])
    }

    pub fn subtype(&mut self, left: &Value, right: &Value) -> Result<bool> {
        let answer = self
            .query(json!({"kind":"typeRelation","relation":"subtype","left":left,"right":right}))?;
        answer["holds"]
            .as_bool()
            .ok_or_else(|| Error::new("missing type relation result"))
    }
}

pub fn run(name: &str, generate: fn(&mut Context<'_>) -> Result<Option<String>>) -> io::Result<()> {
    let mut connection = Connection {
        input: BufReader::new(io::stdin()),
        output: BufWriter::new(io::stdout()),
        next_id: 0,
        exports: HashMap::new(),
    };
    let Some(Message::Request(request)) = Message::read(&mut connection.input)? else {
        return Err(io::Error::other("expected generator/generate request"));
    };
    let batch: Batch = serde_json::from_value(request.params)?;
    if request.method != "generator/generate"
        || batch.protocol_version != PROTOCOL_VERSION
        || batch.generator != name
    {
        return Message::Response(Response::new_err(
            request.id,
            -32602,
            format!("expected a version {PROTOCOL_VERSION} request for {name}"),
        ))
        .write(&mut connection.output);
    }
    let mut result = Generated {
        bodies: vec![],
        diagnostics: vec![],
    };
    for task in batch.tasks {
        for method in &task.methods {
            let mut context = Context {
                connection: &mut connection,
                method,
                owner: &task.owner,
                core: &batch.core_cone,
                types: HashMap::new(),
                members: HashMap::new(),
                imports: Vec::new(),
            };
            match generate(&mut context) {
                Ok(None) => {}
                Ok(Some(source)) => result.bodies.push(GeneratedBody {
                    method: method.method,
                    source: context.with_imports(source),
                }),
                Err(error) => result.diagnostics.push(GeneratorDiagnostic {
                    method: Some(method.method),
                    severity: GeneratorSeverity::Error,
                    code: None,
                    message: error.message,
                    location: Some(error.location.map_or_else(|| method.origin.clone(), |v| *v)),
                    notes: error.notes,
                }),
            }
        }
    }
    Message::Response(Response::new_ok(request.id, result)).write(&mut connection.output)
}
