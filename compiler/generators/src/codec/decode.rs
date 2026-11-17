use super::*;
use crate::record::{Argument, construct};
use crate::source::literal;

impl Codec<'_, '_> {
    pub(super) fn decode(&mut self, body: &mut Body, decoder: &str) -> Result<()> {
        let info = self.check_target()?;
        let target = self.target.clone();
        if info["shape"]["kind"] == "tuple" {
            let result = self.read_tuple(body, &target, decoder)?;
            body.line(format!("return {result}"));
            return Ok(());
        }
        let members = self.ctx.members(&target)?;
        self.check_computed(&members)?;
        let target_name = self.ctx.spelling(&target)?;
        let option = self.ctx.import_type(&["Option"]);
        if info["shape"]["nominalKind"]["kind"] == "enum" {
            let exception = self.ctx.import_type(&["DecodingException"]);
            let outer = body.save(format!("{decoder}.keyed()"));
            let keys = body.save(format!("{outer}.keys"));
            body.line(format!("if ({keys}.size != 1L) {{ throw {exception}({decoder}.path, \"expected exactly one enum variant\") }}"));
            let key = body.save(format!("{keys}[0L]"));
            body.line(format!("when ({key}) {{"));
            let mut names = HashSet::new();
            for variant in &members.variants {
                let wire = self.variant_name(variant, &mut names)?;
                body.line(format!("case {} -> {{", literal(&wire)));
                let child = body.save(format!("{outer}.required({key})"));
                let keyed = variant.style != "positional";
                let args =
                    self.read_fields(body, &child, keyed, &self.variant_slots(&members, variant)?)?;
                body.line(format!("{outer}.end()"));
                let variant_name = format!("{target_name}.{}", ident(&variant.name));
                if variant.style == "unit" {
                    body.line(format!("return {variant_name}"));
                } else {
                    construct(body, &variant_name, keyed, &args, &option);
                }
                body.line("}");
            }
            body.line(format!(
                "else -> {{ throw {exception}({decoder}.path, \"unknown enum variant\") }}\n}}"
            ));
        } else {
            let primary = members.primary(true)?;
            let slots = primary
                .parameters
                .iter()
                .map(|p| members.parameter_slot(p))
                .collect::<Result<Vec<_>>>()?;
            for slot in members.stored_slots()? {
                if !slots.iter().any(|p| p.mapped && p.name == slot.name)
                    && !self.settings(&slot)?.ignore
                {
                    return Err(Error::at(
                        &slot.origin,
                        format!(
                            "automatic decode requires stored property `{}` to be a primary constructor parameter or Serialize(ignore = true)",
                            slot.name
                        ),
                    ));
                }
            }
            let args = self.read_fields(body, decoder, true, &slots)?;
            construct(body, &target_name, true, &args, &option);
        }
        Ok(())
    }

    fn read_fields(
        &mut self,
        body: &mut Body,
        decoder: &str,
        keyed: bool,
        slots: &[Slot],
    ) -> Result<Vec<(String, Argument)>> {
        let mut args = vec![];
        let mut names = HashSet::new();
        let option = self.ctx.import_type(&["Option"]);
        let container = body.save(format!(
            "{decoder}.{}()",
            if keyed { "keyed" } else { "unkeyed" }
        ));
        for slot in slots {
            let config = self.settings(slot)?;
            let nullable = slot.mapped && slot.ty.declaration_type["declaration"] == self.option;
            let key = config.wire(&slot.name);
            let arg = if config.ignore || !slot.mapped {
                if slot.default.is_some() {
                    Argument::Omitted
                } else if nullable {
                    Argument::Supplied(format!("{option}.None"))
                } else {
                    return Err(Error::at(
                        &slot.origin,
                        format!(
                            "automatic decode requires a declared default for omitted constructor parameter `{}`",
                            slot.name
                        ),
                    ));
                }
            } else {
                if keyed && !names.insert(key.to_owned()) {
                    return Err(Error::at(
                        &slot.origin,
                        format!("automatic decode has duplicate field name `{key}`"),
                    ));
                }
                let (strategy, binding) = self
                    .receiver(body, &slot.ty.applied_type)
                    .map_err(|error| error.at_if_missing(&slot.origin))?;
                if keyed && !config.required && (slot.default.is_some() || nullable) {
                    let child = body.name();
                    let field_type = self.ctx.spelling(&slot.ty.applied_type)?;
                    let value = body.name();
                    if slot.default.is_some() {
                        body.line(format!("val {value}: {option}<{field_type}> = when ({container}.optional({})) {{\ncase {option}.Some(val {child}) -> {{ {binding}\n{option}.Some({strategy}.decode({child})) }}\ncase {option}.None -> {option}.None\n}}",literal(key)));
                        Argument::Optional(value)
                    } else {
                        body.line(format!("val {value}: {field_type} = when ({container}.optional({})) {{\ncase {option}.Some(val {child}) -> {{ {binding}\n{strategy}.decode({child}) }}\ncase {option}.None -> {option}.None\n}}",literal(key)));
                        Argument::Supplied(value)
                    }
                } else {
                    let child = body.save(if keyed {
                        format!("{container}.required({})", literal(key))
                    } else {
                        format!("{container}.element()")
                    });
                    body.line(binding);
                    Argument::Supplied(body.save(format!("{strategy}.decode({child})")))
                }
            };
            args.push((slot.name.clone(), arg));
        }
        body.line(format!("{container}.end()"));
        Ok(args)
    }
}
