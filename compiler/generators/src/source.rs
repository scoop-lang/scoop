use crate::client::{Context, Error, Result};
use serde_json::{Value, json};

pub fn ident(name: &str) -> String {
    format!("`{name}`")
}

pub fn literal(text: &str) -> String {
    let mut out = String::from("\"");
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            ch if ch.is_control() => out.push_str(&format!("\\u{{{:x}}}", ch as u32)),
            ch => out.push(ch),
        }
    }
    out.push('"');
    out
}

pub fn application(declaration: &Value, element: &Value) -> Value {
    json!({"kind":"nominalApplication","declaration":declaration,"arguments":[element]})
}
pub fn types(ty: &Value) -> &[Value] {
    ty["arguments"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default()
}
pub fn tuple(items: &[String]) -> String {
    if items.len() == 1 {
        format!("({},)", items[0])
    } else {
        format!("({})", items.join(", "))
    }
}

impl Context<'_> {
    pub fn import_type(&mut self, path: &[impl AsRef<str>]) -> String {
        let name = path
            .iter()
            .map(|part| ident(part.as_ref()))
            .collect::<Vec<_>>()
            .join(".");
        if let Some((_, alias)) = self.imports.iter().find(|(source, _)| *source == name) {
            return alias.clone();
        }
        let alias = format!("generatedType{}", self.imports.len());
        self.imports.push((name, alias.clone()));
        alias
    }

    pub fn with_imports(&self, source: String) -> String {
        if self.imports.is_empty() {
            return source;
        }
        let imports = self
            .imports
            .iter()
            .map(|(path, alias)| format!("import {path} as {alias}"))
            .collect::<Vec<_>>()
            .join("\n");
        format!("{{\n{imports}{}", &source[1..])
    }

    pub fn spelling(&mut self, ty: &Value) -> Result<String> {
        let description = self.describe(ty)?;
        let shape = &description["shape"];
        let text = match shape["kind"].as_str() {
            Some("unit") => self.import_type(&["Unit"]),
            Some("any") => self.import_type(&["Any"]),
            Some("nominal") => {
                let path: Vec<String> = serde_json::from_value(shape["sourcePath"].clone())?;
                let name = match path.as_slice() {
                    [local] if shape["origin"]["source"]["cone"] != self.core => ident(local),
                    _ => self.import_type(&path),
                };
                let arguments = types(ty)
                    .iter()
                    .map(|arg| self.spelling(arg))
                    .collect::<Result<Vec<_>>>()?;
                if arguments.is_empty() {
                    name
                } else {
                    format!("{name}<{}>", arguments.join(", "))
                }
            }
            Some("parameter") => ident(
                shape["parameter"]["name"]
                    .as_str()
                    .ok_or_else(|| Error::new("expected a named type parameter"))?,
            ),
            Some("companion") => format!("{}.Companion", self.spelling(&ty["host"])?),
            Some("tuple") => {
                let elements: Vec<Value> = serde_json::from_value(ty["elements"].clone())?;
                tuple(
                    &elements
                        .iter()
                        .map(|element| self.spelling(element))
                        .collect::<Result<Vec<_>>>()?,
                )
            }
            Some("pointer") => format!(
                "{}<{}>",
                self.import_type(&["Ptr"]),
                self.spelling(&ty["pointee"])?
            ),
            Some("function")
            | Some("nativeFunctionPointer")
            | Some("gcLeafNativeFunctionPointer") => {
                let parameters: Vec<Value> = serde_json::from_value(ty["parameters"].clone())?;
                let parameters = parameters
                    .iter()
                    .map(|p| self.spelling(p))
                    .collect::<Result<Vec<_>>>()?;
                let receiver = if ty["receiver"].is_null() {
                    String::new()
                } else {
                    format!("{}.", self.spelling(&ty["receiver"])?)
                };
                let effect = if ty["effect"] == "suspend" {
                    "suspend "
                } else {
                    ""
                };
                let signature = format!(
                    "{effect}{receiver}({}) -> {}",
                    parameters.join(", "),
                    self.spelling(&ty["result"])?
                );
                match shape["kind"].as_str() {
                    Some("nativeFunctionPointer") => {
                        format!("{}<{signature}>", self.import_type(&["FunPtr"]))
                    }
                    Some("gcLeafNativeFunctionPointer") => {
                        format!("{}<{signature}>", self.import_type(&["GCLeafFunPtr"]))
                    }
                    _ => signature,
                }
            }
            _ => return Err(Error::new("type cannot be named in the generated method")),
        };
        Ok(text)
    }
}

// Local names are scoped to the generated body. A fresh sequence also keeps nested
// tuple codecs and enum arms independent of the interface's parameter names.
#[derive(Default)]
pub struct Body {
    pub lines: Vec<String>,
    next: usize,
}
impl Body {
    pub fn name(&mut self) -> String {
        let index = self.next;
        self.next += 1;
        format!("generated{index}")
    }
    pub fn line(&mut self, text: impl Into<String>) {
        self.lines.push(text.into());
    }
    pub fn save(&mut self, expr: impl AsRef<str>) -> String {
        let name = self.name();
        self.line(format!("val {name} = {}", expr.as_ref()));
        name
    }
    pub fn finish(self) -> String {
        format!("{{\n{}\n}}", self.lines.join("\n"))
    }
}
