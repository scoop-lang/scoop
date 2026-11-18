use super::*;
use crate::source::literal;

impl Codec<'_, '_> {
    pub(super) fn encode(&mut self, body: &mut Body, value: &str, encoder: &str) -> Result<()> {
        let info = self.check_target()?;
        let target = self.target.clone();
        if info["shape"]["kind"] == "tuple" {
            return self.write_tuple(body, &target, value, encoder);
        }
        let members = self.ctx.members(&target)?;
        self.check_computed(&members)?;
        let outer = body.save(format!("{encoder}.keyed()"));
        if info["shape"]["nominalKind"]["kind"] == "enum" {
            let alias = body.name();
            body.line(format!(
                "typealias {alias} = {}",
                self.ctx.spelling(&target)?
            ));
            body.line(format!("when ({value}) {{"));
            let mut names = HashSet::new();
            for variant in &members.variants {
                let wire = self.variant_name(variant, &mut names)?;
                let slots = self.variant_slots(&members, variant)?;
                let mut bindings = vec![];
                let mut patterns = vec![];
                for slot in &slots {
                    let name = body.name();
                    let pattern = if self.settings(slot)?.ignore {
                        "_".to_owned()
                    } else {
                        format!("val {name}")
                    };
                    patterns.push(if variant.style == "named" {
                        format!("{}: {pattern}", ident(&slot.name))
                    } else {
                        pattern
                    });
                    bindings.push(name);
                }
                let payload = match variant.style.as_str() {
                    "unit" => String::new(),
                    "named" => format!(" {{ {} }}", patterns.join(", ")),
                    _ => format!("({})", patterns.join(", ")),
                };
                body.line(format!(
                    "case {alias}.{}{payload} -> {{",
                    ident(&variant.name)
                ));
                let child = body.save(format!("{outer}.field({})", literal(&wire)));
                let keyed = variant.style != "positional";
                let kind = if keyed { "keyed" } else { "unkeyed" };
                let fields = body.save(format!("{child}.{kind}()"));
                self.write_fields(body, &fields, keyed, slots.into_iter().zip(bindings))?;
                body.line(format!("{fields}.end()\n}}"));
            }
            body.line("}");
        } else {
            let slots = members.stored_slots()?.into_iter().map(|slot| {
                let access = format!("{value}.{}", ident(&slot.name));
                (slot, access)
            });
            self.write_fields(body, &outer, true, slots)?;
        }
        body.line(format!("{outer}.end()"));
        Ok(())
    }

    fn write_fields(
        &mut self,
        body: &mut Body,
        container: &str,
        keyed: bool,
        fields: impl IntoIterator<Item = (Slot, String)>,
    ) -> Result<()> {
        let mut names = HashSet::new();
        for (slot, access) in fields {
            let config = self.settings(&slot)?;
            if config.ignore {
                continue;
            }
            if !slot.plain_storage || !slot.readable {
                return Err(Error::at(
                    &slot.origin,
                    format!(
                        "automatic encode requires readable storage for `{}`; provide an explicit encode implementation or mark it Serialize(ignore = true)",
                        slot.name
                    ),
                ));
            }
            let key = config.wire(&slot.name);
            if keyed && !names.insert(key.to_owned()) {
                return Err(Error::at(
                    &slot.origin,
                    format!("automatic encode has duplicate field name `{key}`"),
                ));
            }
            let value = body.save(access);
            let omit =
                keyed && !config.required && !config.emit_null && self.directly_none(&slot)?;
            if omit {
                let option = self.ctx.import_type(&["Option"]);
                body.line(format!(
                    "when ({value}) {{\ncase {option}.None -> {{}}\ncase {option}.Some(_) -> {{"
                ));
            }
            let child = if keyed {
                body.save(format!("{container}.field({})", literal(key)))
            } else {
                body.save(format!("{container}.element()"))
            };
            let (strategy, binding) = self
                .receiver(body, &slot.ty.applied_type)
                .map_err(|error| error.at_if_missing(&slot.origin))?;
            body.line(binding);
            body.line(format!("{strategy}.encode({value}, {child})"));
            if omit {
                body.line("}\n}");
            }
        }
        Ok(())
    }
}
