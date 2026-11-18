use super::*;
use crate::source::tuple;
use serde_json::json;

impl Codec<'_, '_> {
    pub(super) fn strategy(&mut self, ty: &Value) -> Result<String> {
        let interface = application(&self.protocol, ty);
        let mut matches = Vec::new();
        for dependency in &self.dependencies {
            if self.ctx.subtype(&dependency.ty.applied_type, &interface)? {
                matches.push(&dependency.name);
            }
        }
        match matches.as_slice() {
            [name] => return Ok(format!("this.{}", ident(name))),
            [] => {}
            _ => {
                return Err(Error::new(format!(
                    "ambiguous {} dependency for {}: {}",
                    self.direction.method(),
                    self.ctx.describe(ty)?["name"].as_str().unwrap_or("?"),
                    matches
                        .iter()
                        .map(|s| s.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                )));
            }
        }
        if *ty == self.target {
            return Ok("this".into());
        }
        let info = self.ctx.describe(ty)?;
        if info["shape"]["kind"] == "unit" {
            let helper = if self.direction == Direction::Encode {
                "UnitEncoder"
            } else {
                "UnitDecoder"
            };
            return Ok(self.ctx.import_type(&[helper]));
        }
        if info["shape"]["kind"] == "tuple" {
            let mut body = Body::default();
            let adapter = match self.direction {
                Direction::Encode => {
                    self.write_tuple(&mut body, ty, "tupleValue", "tupleEncoder")?;
                    "ValueEncoderFunction"
                }
                Direction::Decode => {
                    let expr = self.read_tuple(&mut body, ty, "tupleDecoder")?;
                    body.line(expr);
                    "ValueDecoderFunction"
                }
            };
            let params = if self.direction == Direction::Encode {
                "tupleValue, tupleEncoder"
            } else {
                "tupleDecoder"
            };
            let adapter = self.ctx.import_type(&[adapter]);
            return Ok(format!(
                "{adapter}<{}> {{ {params} ->\n{}\n}}",
                self.ctx.spelling(ty)?,
                body.lines.join("\n")
            ));
        }
        if let [element] = types(ty) {
            for container in ["Option", "Array", "MutableArray", "ArrayList"] {
                if ty["declaration"] == self.ctx.core_id(container)? {
                    let element = self.strategy(element)?;
                    let factory = if self.direction == Direction::Encode {
                        "encoder"
                    } else {
                        "decoder"
                    };
                    return Ok(format!(
                        "{}.Companion.{factory}({element})",
                        self.ctx.spelling(ty)?
                    ));
                }
            }
        }
        if !info["visibleCompanion"].is_null()
            && self.ctx.subtype(&info["visibleCompanion"], &interface)?
        {
            return Ok(format!("{}.Companion", self.ctx.spelling(ty)?));
        }
        Err(Error::new(format!(
            "no field {} for {}: provide a primary val dependency, a conforming companion, or an explicit {} implementation",
            self.direction.protocol(),
            info["name"].as_str().unwrap_or("?"),
            self.direction.method()
        )))
    }

    /// Bind a selected value to the exact interface requirement at its evaluation site.
    /// Inherited conditions can establish conformance of a constructed companion.
    pub(super) fn receiver(&mut self, body: &mut Body, ty: &Value) -> Result<(String, String)> {
        let name = body.name();
        let field = self.ctx.spelling(ty)?;
        let expression = self.strategy(ty)?;
        let binding = format!(
            "val {name}: {}<{field}> = {expression}",
            self.ctx.import_type(&[self.direction.protocol()])
        );
        Ok((name, binding))
    }

    pub(super) fn write_tuple(
        &mut self,
        body: &mut Body,
        ty: &Value,
        value: &str,
        encoder: &str,
    ) -> Result<()> {
        let elements: Vec<Value> = serde_json::from_value(ty["elements"].clone())?;
        let container = body.save(format!("{encoder}.unkeyed()"));
        for (i, element) in elements.iter().enumerate() {
            let field = body.save(format!("{value}._{}", i + 1));
            let child = body.save(format!("{container}.element()"));
            let (receiver, binding) = self.receiver(body, element)?;
            body.line(binding);
            body.line(format!("{receiver}.encode({field}, {child})"));
        }
        body.line(format!("{container}.end()"));
        Ok(())
    }

    pub(super) fn read_tuple(
        &mut self,
        body: &mut Body,
        ty: &Value,
        decoder: &str,
    ) -> Result<String> {
        let elements: Vec<Value> = serde_json::from_value(ty["elements"].clone())?;
        let container = body.save(format!("{decoder}.unkeyed()"));
        let mut values = vec![];
        for element in elements {
            let child = body.save(format!("{container}.element()"));
            let (receiver, binding) = self.receiver(body, &element)?;
            body.line(binding);
            values.push(body.save(format!("{receiver}.decode({child})")));
        }
        body.line(format!("{container}.end()"));
        Ok(tuple(&values))
    }

    pub(super) fn directly_none(&mut self, slot: &Slot) -> Result<bool> {
        if slot.ty.declaration_type["declaration"] != self.option {
            return Ok(false);
        }
        let Some(reference) = &slot.default else {
            return Ok(true);
        };
        let response = self.ctx.query(json!({"kind":"describeDefaults","defaults":[{"owner":reference["owner"],"parameter":reference["parameter"]}]}))?;
        let default = &response["defaults"][0]["definition"]["body"];
        let op = &default["value"]["operation"];
        Ok(default["statements"].as_array().is_some_and(Vec::is_empty)
            && (op["kind"] == "noneLiteral"
                || (op["kind"] == "variantConstruct"
                    && op["arguments"].as_array().is_some_and(Vec::is_empty))))
    }
}
