use serde::{Deserialize, Serialize};

pub const SCHEMA: u32 = 1;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Index {
    pub schema: u32,
    pub target: String,
    pub cones: Vec<ConeReference>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConeReference {
    pub key: String,
    pub coordinate: String,
    /// Relative to the site root, like every documentation URL.
    pub file: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Cone {
    pub schema: u32,
    pub coordinate: String,
    pub declarations: Vec<Declaration>,
    pub bindings: Vec<Binding>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Declaration {
    pub key: String,
    pub alternate_keys: Vec<String>,
    pub parent: Option<String>,
    pub package: Vec<String>,
    pub name: String,
    pub qualified_name: String,
    pub kind: String,
    pub visibility: String,
    pub property: Option<Property>,
    pub signature: String,
    pub receiver: Option<String>,
    pub equality_requirements: Option<Vec<String>>,
    pub aliases: Vec<String>,
    pub source: Source,
    pub url: String,
    pub documentation: Option<Documentation>,
    pub relations: Vec<Relation>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    pub path: Option<String>,
    pub line: usize,
    pub byte_start: u32,
    pub byte_end: u32,
    pub url: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Documentation {
    pub markdown: String,
    pub tags: Vec<Tag>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Tag {
    pub kind: String,
    pub label: String,
    pub markdown: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Relation {
    pub kind: String,
    pub signature: String,
    pub origin: String,
    pub url: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Property {
    pub getter: Accessor,
    pub setter: Option<Accessor>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Accessor {
    pub visibility: String,
    pub hidden: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Binding {
    pub package: Vec<String>,
    pub name: String,
    pub target: String,
    pub reexport: bool,
}

impl Binding {
    pub fn path(&self) -> String {
        self.package
            .iter()
            .chain(std::iter::once(&self.name))
            .map(|part| format!("`{part}`"))
            .collect::<Vec<_>>()
            .join(".")
    }
}
