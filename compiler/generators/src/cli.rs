use crate::client::{self, Context, Error, Result};
use crate::record::{Argument, Slot, construct};
use crate::source::{Body, ident, literal, types};
use serde_json::Value;
use std::collections::HashSet;

mod options;
mod parsers;

pub fn run() -> std::io::Result<()> {
    client::run("scoop.cli", generate)
}

struct Cli<'a, 'b> {
    ctx: &'a mut Context<'b>,
    library: String,
    option: Value,
    parser: Value,
    dependencies: Vec<Slot>,
}

impl Cli<'_, '_> {
    fn name(&mut self, name: &str) -> String {
        self.ctx.import_type(&["scoop", "cli", name])
    }
    fn export(&mut self, name: &str) -> Result<Value> {
        self.ctx.export(&self.library, &["scoop", "cli", name])
    }
}

fn generate(ctx: &mut Context<'_>) -> Result<Option<String>> {
    let binding = ctx.method.signature.name == "bind";
    if !binding && ctx.method.signature.name != "entries" {
        return Ok(None);
    }
    let mut selected = None;
    for req in &ctx.method.requirements {
        let info = ctx.describe(&req.interface)?;
        let shape = &info["shape"];
        if shape["sourcePath"] != serde_json::json!(["scoop", "cli", "CliArguments"]) {
            continue;
        }
        let provider = shape["origin"]["source"]["cone"]
            .as_str()
            .ok_or_else(|| Error::new("missing CliArguments declaration origin"))?;
        if ctx.export(provider, &["scoop", "cli", "CliArguments"])? == req.interface["declaration"]
            && let [ty] = types(&req.interface)
        {
            selected = Some((provider.to_owned(), ty.clone()));
            break;
        }
    }
    let Some((library, target)) = selected else {
        return Ok(None);
    };
    let option = ctx.core_id("Option")?;
    let parser = ctx.export(&library, &["scoop", "cli", "CliValueParser"])?;
    let owner = ctx.members(ctx.owner)?;
    let mut dependencies = vec![];
    if let Ok(primary) = owner.primary(false) {
        for parameter in &primary.parameters {
            let slot = owner.parameter_slot(parameter)?;
            if slot.mapped && !slot.mutable {
                dependencies.push(slot)
            }
        }
    }
    let mut cli = Cli {
        ctx,
        library,
        parser,
        option,
        dependencies,
    };
    let shape = cli.ctx.describe(&target)?["shape"]["nominalKind"].clone();
    if !(shape["kind"] == "struct"
        || (shape["kind"] == "class"
            && shape["modality"] == "final"
            && shape["baseClass"].is_null()))
    {
        return Err(Error::new(
            "automatic CLI arguments require a struct or final class without a class base",
        ));
    }
    let members = cli.ctx.members(&target)?;
    let primary = members.primary(binding)?;
    let slots = primary
        .parameters
        .iter()
        .map(|p| members.parameter_slot(p))
        .collect::<Result<Vec<_>>>()?;
    for stored in members.stored_slots()? {
        if !stored.plain_storage || !slots.iter().any(|s| s.mapped && s.name == stored.name) {
            return Err(Error::at(
                &stored.origin,
                format!(
                    "CLI member `{}` requires ordinary primary constructor storage",
                    stored.name
                ),
            ));
        }
    }
    let mut body = Body::default();
    let mut args = vec![];
    let mut entries = vec![];
    let mut aliases = HashSet::from(["--help".to_owned(), "-h".to_owned()]);
    let mut positions = vec![];
    let entry_ty = cli.name("CliEntry");
    let role_ty = cli.name("CliRole");
    let shape_ty = cli.name("CliValueShape");
    let parser_ty = cli.name("CliValueParser");
    let option = cli.ctx.import_type(&["Option"]);
    let values = ctx_name(cli.ctx, binding)?;
    for slot in slots {
        if !slot.mapped {
            return Err(Error::at(
                &slot.origin,
                format!(
                    "CLI constructor parameter `{}` has no corresponding member",
                    slot.name
                ),
            ));
        }
        let settings = cli.options(&slot)?;
        let nullable = slot.ty.declaration_type["declaration"] == cli.option;
        let has_default = slot.default.is_some();
        if settings.ignore {
            let arg = if has_default {
                Argument::Omitted
            } else if nullable {
                Argument::Supplied(format!("{option}.None"))
            } else {
                return Err(Error::at(
                    &slot.origin,
                    "CliIgnore requires a constructor default or a field declared as Option",
                ));
            };
            args.push((slot.name, arg));
            continue;
        }
        let parser = cli.field_parser(&slot.ty.applied_type, &slot.origin)?;
        let required = settings.required || (!nullable && !has_default);
        let role = match settings.role {
            options::Role::Flag => format!("{role_ty}.Flag"),
            options::Role::Option => format!("{role_ty}.Option"),
            options::Role::Argument(index) => {
                positions.push((index, required, parser.repeated, slot.origin.clone()));
                format!("{role_ty}.Argument({index}L)")
            }
        };
        for alias in settings.names.iter().chain(&settings.negative) {
            if !options::valid_name(alias) || !aliases.insert(alias.clone()) {
                return Err(Error::at(
                    &slot.origin,
                    format!("invalid or duplicate CLI option name `{alias}`"),
                ));
            }
        }
        let key = literal(&slot.name);
        let field_type = cli.ctx.spelling(&slot.ty.applied_type)?;
        let receiver = body.name();
        let binding_source = format!(
            "val {receiver}: {parser_ty}<{field_type}> = {}",
            parser.expression
        );
        if binding {
            let arg = if required {
                let field = body.save(format!("{values}.required({key})"));
                body.line(binding_source);
                Argument::Supplied(body.save(format!("{receiver}.parseValue({field})")))
            } else {
                let value = body.name();
                let field = body.name();
                if has_default {
                    body.line(format!("val {value}: {option}<{field_type}> = when ({values}.optional({key})) {{\ncase {option}.Some(val {field}) -> {{ {binding_source}\n{option}.Some({receiver}.parseValue({field})) }}\ncase {option}.None -> {option}.None\n}}"));
                    Argument::Optional(value)
                } else {
                    body.line(format!("val {value}: {field_type} = when ({values}.optional({key})) {{\ncase {option}.Some(val {field}) -> {{ {binding_source}\n{receiver}.parseValue({field}) }}\ncase {option}.None -> {option}.None\n}}"));
                    Argument::Supplied(value)
                }
            };
            args.push((slot.name, arg));
        } else {
            let shape = if matches!(settings.role, options::Role::Flag) {
                format!("{shape_ty}.Flag")
            } else {
                body.line(binding_source);
                format!("{receiver}.shape()")
            };
            let label = if settings.label.is_empty() {
                &parser.label
            } else {
                &settings.label
            };
            entries.push(format!(
                "{entry_ty}({key}, {role}, {shape}, {}, {}, {}, {}, {}, {required}, {has_default})",
                strings(&settings.names),
                strings(&settings.negative),
                literal(&settings.help),
                literal(label),
                literal(&settings.default_text)
            ));
        }
    }
    options::validate_positions(positions)?;
    if binding {
        construct(&mut body, &cli.ctx.spelling(&target)?, true, &args, &option);
    } else {
        body.line(format!("return [{}]", entries.join(",\n")))
    }
    Ok(Some(body.finish()))
}

fn strings(values: &[String]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(|s| literal(s))
            .collect::<Vec<_>>()
            .join(", ")
    )
}
fn ctx_name(ctx: &Context<'_>, binding: bool) -> Result<String> {
    match (binding, ctx.method.signature.parameters.as_slice()) {
        (true, [values]) => Ok(ident(&values.name)),
        (false, []) => Ok(String::new()),
        _ => Err(Error::new("invalid CliArguments method signature")),
    }
}
