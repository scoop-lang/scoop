use crate::client::{self, Context, Error, Result};
use crate::model::{Annotation, ConstructorKind, Members, Variant};
use crate::record::Slot;
use crate::source::{Body, application, ident, types};
use serde_json::Value;
use std::collections::HashSet;

mod decode;
mod encode;
mod selection;

pub fn run() -> std::io::Result<()> {
    client::run("scoop.codec", generate)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Direction {
    Encode,
    Decode,
}
impl Direction {
    fn method(self) -> &'static str {
        match self {
            Self::Encode => "encode",
            Self::Decode => "decode",
        }
    }
    fn protocol(self) -> &'static str {
        match self {
            Self::Encode => "ValueEncoder",
            Self::Decode => "ValueDecoder",
        }
    }
}

struct Codec<'a, 'b> {
    ctx: &'a mut Context<'b>,
    direction: Direction,
    target: Value,
    protocol: Value,
    option: Value,
    serialize: Value,
    dependencies: Vec<Slot>,
}

#[derive(Default)]
struct Settings {
    rename: String,
    required: bool,
    emit_null: bool,
    ignore: bool,
}

impl Settings {
    fn read(annotations: &[Annotation], id: &Value, variant: bool, optional: bool) -> Result<Self> {
        let Some(annotation) = annotations.iter().find(|a| a.annotation == *id) else {
            return Ok(Self::default());
        };
        let [rename, required, emit_null, ignore] = annotation.arguments.as_slice() else {
            return Err(Error::at(
                &annotation.origin,
                "expected the core Serialize annotation parameters",
            ));
        };
        let value = Self {
            rename: serde_json::from_value(rename["value"]["value"].clone())?,
            required: serde_json::from_value(required["value"]["value"].clone())?,
            emit_null: serde_json::from_value(emit_null["value"]["value"].clone())?,
            ignore: serde_json::from_value(ignore["value"]["value"].clone())?,
        };
        let error =
            if value.ignore && (!value.rename.is_empty() || value.emit_null || value.required) {
                Some("Serialize ignore cannot be combined with rename, required, or emitNull")
            } else if variant && (value.ignore || value.required || value.emit_null) {
                Some("Serialize on an enum variant only accepts rename")
            } else if value.emit_null && !optional {
                Some("Serialize emitNull requires a field declared as core Option")
            } else {
                None
            };
        if let Some(error) = error {
            return Err(Error::at(&annotation.origin, error));
        }
        Ok(value)
    }
    fn wire<'a>(&'a self, name: &'a str) -> &'a str {
        if self.rename.is_empty() {
            name
        } else {
            &self.rename
        }
    }
}

fn generate(ctx: &mut Context<'_>) -> Result<Option<String>> {
    let direction = match ctx.method.signature.name.as_str() {
        "encode" => Direction::Encode,
        "decode" => Direction::Decode,
        _ => return Ok(None),
    };
    let protocol = ctx.core_id(direction.protocol())?;
    let Some(interface) = ctx
        .method
        .requirements
        .iter()
        .map(|r| &r.interface)
        .find(|ty| ty["declaration"] == protocol)
    else {
        return Ok(None);
    };
    let [target] = types(interface) else {
        return Err(Error::new("expected one codec target"));
    };
    let target = target.clone();
    let option = ctx.core_id("Option")?;
    let serialize = ctx.core_id("Serialize")?;
    let owner = ctx.owner.clone();
    let members = ctx.members(&owner)?;
    let mut dependencies = vec![];
    if let Ok(primary) = members.primary(false) {
        for param in &primary.parameters {
            let slot = members.parameter_slot(param)?;
            if slot.mapped && !slot.mutable {
                dependencies.push(slot);
            }
        }
    }
    let mut codec = Codec {
        ctx,
        direction,
        target,
        protocol,
        option,
        serialize,
        dependencies,
    };
    let mut body = Body::default();
    let parameters = &codec.ctx.method.signature.parameters;
    match (direction, parameters.as_slice()) {
        (Direction::Encode, [value, encoder]) => {
            codec.encode(&mut body, &ident(&value.name), &ident(&encoder.name))?
        }
        (Direction::Decode, [decoder]) => codec.decode(&mut body, &ident(&decoder.name))?,
        _ => return Err(Error::new("invalid codec method signature")),
    }
    Ok(Some(body.finish()))
}

impl Codec<'_, '_> {
    fn settings(&self, slot: &Slot) -> Result<Settings> {
        Settings::read(
            &slot.annotations,
            &self.serialize,
            false,
            slot.ty.declaration_type["declaration"] == self.option,
        )
    }
    fn variant_name(&self, variant: &Variant, seen: &mut HashSet<String>) -> Result<String> {
        let config = Settings::read(&variant.annotations, &self.serialize, true, false)?;
        let name = config.wire(&variant.name).to_owned();
        if !seen.insert(name.clone()) {
            return Err(Error::at(
                &variant.origin,
                format!("duplicate variant name `{name}`"),
            ));
        }
        Ok(name)
    }
    fn check_target(&mut self) -> Result<Value> {
        let info = self.ctx.describe(&self.target)?;
        let shape = &info["shape"];
        match (
            shape["kind"].as_str(),
            shape["nominalKind"]["kind"].as_str(),
        ) {
            (Some("tuple"), _) | (_, Some("enum" | "struct")) => Ok(info),
            (_, Some("class"))
                if shape["nominalKind"]["modality"] == "final"
                    && shape["nominalKind"]["baseClass"].is_null() =>
            {
                Ok(info)
            }
            _ => Err(Error::new(format!(
                "automatic {} requires a struct, enum, tuple, or final class without a class base; target {} requires an explicit implementation",
                self.direction.method(),
                info["name"].as_str().unwrap_or("?")
            ))),
        }
    }
    fn check_computed(&self, members: &Members) -> Result<()> {
        for property in &members.properties {
            if let Some(annotation) = property
                .annotations
                .iter()
                .find(|a| a.annotation == self.serialize)
                && !members
                    .fields
                    .iter()
                    .any(|f| f.storage.property() == Some(property.declaration))
            {
                return Err(Error::at(
                    &annotation.origin,
                    "Serialize requires a stored property",
                ));
            }
        }
        Ok(())
    }
    fn variant_slots(&self, members: &Members, variant: &Variant) -> Result<Vec<Slot>> {
        let constructor = members.constructors.iter().find(|c| {
            c.constructor_kind
                == ConstructorKind::Variant {
                    variant: variant.declaration,
                }
        });
        variant
            .fields
            .iter()
            .enumerate()
            .map(|(i, field)| {
                if variant.style == "positional"
                    && let Some(annotation) = field
                        .annotations
                        .iter()
                        .find(|a| a.annotation == self.serialize)
                {
                    return Err(Error::at(
                        &annotation.origin,
                        "Serialize requires a named field",
                    ));
                }
                Ok(Slot::field(
                    field,
                    constructor.and_then(|c| c.parameters.get(i)),
                ))
            })
            .collect()
    }
}
