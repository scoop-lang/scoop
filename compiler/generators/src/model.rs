use scoop_generator_protocol::{FieldId, Reference, SourceLocation};
use scoop_identity::{PersistentEnumVariantId, PersistentPropertyId};
use serde::Deserialize;
use serde_json::Value;

#[derive(Clone, Deserialize)]
pub struct Members {
    pub fields: Vec<Field>,
    pub properties: Vec<Property>,
    pub variants: Vec<Variant>,
    pub constructors: Vec<Constructor>,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TypeUse {
    pub declaration_type: Value,
    pub applied_type: Value,
}

#[derive(Clone, Deserialize)]
pub struct Annotation {
    pub annotation: Value,
    pub arguments: Vec<Value>,
    pub origin: SourceLocation,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Field {
    pub declaration: FieldId,
    pub name: String,
    pub value_type: TypeUse,
    pub storage: Storage,
    pub annotations: Vec<Annotation>,
    pub origin: SourceLocation,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Storage {
    Declared,
    PropertyBacking {
        property: Reference<PersistentPropertyId>,
    },
    PropertyDelegate {
        property: Reference<PersistentPropertyId>,
    },
    EnclosingInstance,
    CapturedEnvironment,
    Generated,
}

impl Storage {
    pub fn property(self) -> Option<Reference<PersistentPropertyId>> {
        match self {
            Self::PropertyBacking { property } | Self::PropertyDelegate { property } => {
                Some(property)
            }
            _ => None,
        }
    }
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Property {
    pub declaration: Reference<PersistentPropertyId>,
    pub name: String,
    pub value_type: TypeUse,
    pub getter: Accessor,
    pub setter: Option<Accessor>,
    pub annotations: Vec<Annotation>,
    pub origin: SourceLocation,
}

#[derive(Clone, Deserialize)]
pub struct Accessor {
    pub access: Access,
    pub implementation: String,
}

impl Accessor {
    pub fn is_storage(&self) -> bool {
        matches!(self.implementation.as_str(), "storage" | "storageBody")
    }
}

#[derive(Clone, Deserialize)]
pub struct Access {
    pub accessible: bool,
}

#[derive(Clone, Deserialize)]
pub struct Variant {
    pub declaration: Reference<PersistentEnumVariantId>,
    pub name: String,
    pub style: String,
    pub fields: Vec<Field>,
    pub annotations: Vec<Annotation>,
    pub origin: SourceLocation,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Constructor {
    pub constructor_kind: ConstructorKind,
    pub parameters: Vec<Parameter>,
    pub access: Access,
    pub origin: SourceLocation,
}
#[derive(Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ConstructorKind {
    Primary,
    Secondary,
    Variant {
        variant: Reference<PersistentEnumVariantId>,
    },
}
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Parameter {
    pub name: String,
    pub value_type: TypeUse,
    pub calling: Value,
    pub target: Target,
    pub origin: SourceLocation,
}

impl Parameter {
    pub fn default(&self) -> Option<&Value> {
        self.calling
            .get("default")
            .or_else(|| self.calling["omission"].get("default"))
    }
}
#[derive(Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Target {
    Field {
        field: FieldId,
    },
    Property {
        property: Reference<PersistentPropertyId>,
    },
    Plain,
}
