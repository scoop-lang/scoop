use crate::client::{Context, Error, Result};
use crate::model::{
    Annotation, Constructor, ConstructorKind, Field, Members, Parameter, Property, Storage, Target,
    TypeUse,
};
use crate::source::{Body, ident};
use scoop_generator_protocol::SourceLocation;
use serde_json::Value;

mod defaults;
use defaults::SimpleDefault;

#[cfg(test)]
mod tests;

#[derive(Clone)]
pub struct Slot {
    pub name: String,
    pub ty: TypeUse,
    pub annotations: Vec<Annotation>,
    pub origin: SourceLocation,
    pub default: Option<Value>,
    pub mapped: bool,
    pub readable: bool,
    pub plain_storage: bool,
    pub mutable: bool,
}

impl Slot {
    pub fn field(field: &Field, parameter: Option<&Parameter>) -> Self {
        Self {
            name: field.name.clone(),
            ty: field.value_type.clone(),
            annotations: field.annotations.clone(),
            origin: field.origin.clone(),
            default: parameter.and_then(Parameter::default).cloned(),
            mapped: true,
            readable: true,
            plain_storage: true,
            mutable: false,
        }
    }
    fn property(property: &Property, parameter: Option<&Parameter>, delegated: bool) -> Self {
        Self {
            name: property.name.clone(),
            ty: property.value_type.clone(),
            annotations: property.annotations.clone(),
            origin: property.origin.clone(),
            default: parameter.and_then(Parameter::default).cloned(),
            mapped: true,
            readable: property.getter.access.accessible,
            mutable: property.setter.is_some(),
            plain_storage: !delegated
                && property.getter.is_storage()
                && property
                    .setter
                    .as_ref()
                    .is_none_or(crate::model::Accessor::is_storage),
        }
    }
}

impl Members {
    pub fn primary(&self, constructing: bool) -> Result<&Constructor> {
        let primary = self
            .constructors
            .iter()
            .find(|ctor| ctor.constructor_kind == ConstructorKind::Primary)
            .ok_or_else(|| Error::new("automatic construction requires a primary constructor"))?;
        if constructing {
            if !primary.access.accessible {
                return Err(Error::new(
                    "the primary constructor is inaccessible from this implementation",
                )
                .note(&primary.origin, "primary constructor declared here"));
            }
            if let Some(other) = self.constructors.iter().find(|ctor| {
                ctor.access.accessible && ctor.constructor_kind == ConstructorKind::Secondary
            }) {
                return Err(Error::new("automatic construction requires the primary constructor to be the only visible constructor")
                    .note(&other.origin, "move this construction entry to a named companion factory"));
            }
        }
        Ok(primary)
    }

    pub fn stored_slots(&self) -> Result<Vec<Slot>> {
        let parameters = self
            .primary(false)
            .ok()
            .map(|primary| primary.parameters.as_slice())
            .unwrap_or_default();
        let mut slots = Vec::with_capacity(self.fields.len());
        for field in &self.fields {
            let slot = match field.storage {
                Storage::Declared => {
                    let parameter = parameters.iter().find(|p| {
                        matches!(p.target, Target::Field {field: id} if id == field.declaration)
                    });
                    Slot::field(field, parameter)
                }
                Storage::PropertyBacking { property } | Storage::PropertyDelegate { property } => {
                    let item = self
                        .properties
                        .iter()
                        .find(|p| p.declaration == property)
                        .ok_or_else(|| {
                            Error::at(&field.origin, "missing property for stored field")
                        })?;
                    let parameter = parameters.iter().find(
                        |p| matches!(p.target, Target::Property { property: id } if id == property),
                    );
                    let delegated = matches!(field.storage, Storage::PropertyDelegate { .. });
                    Slot::property(item, parameter, delegated)
                }
                Storage::EnclosingInstance | Storage::CapturedEnvironment | Storage::Generated => {
                    return Err(Error::at(
                        &field.origin,
                        "automatic generation requires a record with ordinary declared storage",
                    ));
                }
            };
            slots.push(slot);
        }
        Ok(slots)
    }

    pub fn parameter_slot(&self, parameter: &Parameter) -> Result<Slot> {
        let missing = || {
            Error::at(
                &parameter.origin,
                "constructor parameter has no corresponding member",
            )
        };
        match &parameter.target {
            Target::Field { field } => {
                let field = self
                    .fields
                    .iter()
                    .find(|f| f.declaration == *field)
                    .ok_or_else(missing)?;
                Ok(Slot::field(field, Some(parameter)))
            }
            Target::Property { property } => {
                let item = self
                    .properties
                    .iter()
                    .find(|p| &p.declaration == property)
                    .ok_or_else(missing)?;
                let delegated = self.fields.iter().any(|f| {
                    matches!(f.storage, Storage::PropertyDelegate { property: id } if id == *property)
                });
                Ok(Slot::property(item, Some(parameter), delegated))
            }
            Target::Plain => Ok(Slot {
                name: parameter.name.clone(),
                ty: parameter.value_type.clone(),
                annotations: vec![],
                origin: parameter.origin.clone(),
                default: parameter.default().cloned(),
                mapped: false,
                readable: false,
                plain_storage: false,
                mutable: false,
            }),
        }
    }
}

pub enum Argument {
    Supplied(String),
    Optional(String),
    Defaulted {
        presence: String,
        value_type: String,
        default: SimpleDefault,
    },
    Omitted,
}

impl Argument {
    pub fn optional(
        ctx: &mut Context<'_>,
        slot: &Slot,
        presence: String,
        value_type: String,
        option: &Value,
    ) -> Result<Self> {
        Ok(match defaults::read(ctx, slot, option)? {
            Some(default) => Self::Defaulted {
                presence,
                value_type,
                default,
            },
            None => Self::Optional(presence),
        })
    }
}

pub fn construct(
    body: &mut Body,
    target: &str,
    named: bool,
    args: &[(String, Argument)],
    option: &str,
) {
    fn emit(
        body: &mut Body,
        target: &str,
        named: bool,
        args: &[(String, Argument)],
        mut actual: Vec<String>,
        option: &str,
    ) {
        for (index, (name, arg)) in args.iter().enumerate() {
            let prefix = if named {
                format!("{} = ", ident(name))
            } else {
                String::new()
            };
            match arg {
                Argument::Supplied(value) => actual.push(format!("{prefix}{value}")),
                Argument::Omitted => {}
                Argument::Defaulted {
                    presence,
                    value_type,
                    default,
                } => {
                    let value = body.name();
                    let provided = body.name();
                    let fallback = default.source(option);
                    body.line(format!(
                        "val {value}: {value_type} = when ({presence}) {{\ncase {option}.Some(val {provided}) -> {provided}\ncase {option}.None -> {fallback}\n}}"
                    ));
                    actual.push(format!("{prefix}{value}"));
                }
                Argument::Optional(presence) => {
                    let value = body.name();
                    body.line(format!(
                        "when ({presence}) {{\ncase {option}.Some(val {value}) -> {{"
                    ));
                    let mut provided = actual.clone();
                    provided.push(format!("{prefix}{value}"));
                    emit(body, target, named, &args[index + 1..], provided, option);
                    body.line(format!("}}\ncase {option}.None -> {{"));
                    emit(body, target, named, &args[index + 1..], actual, option);
                    body.line("}\n}");
                    return;
                }
            }
        }
        body.line(format!("return {target}({})", actual.join(", ")));
    }
    emit(body, target, named, args, vec![], option);
}
