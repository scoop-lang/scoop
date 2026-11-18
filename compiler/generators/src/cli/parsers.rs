use super::*;
use crate::source::application;
use scoop_generator_protocol::SourceLocation;

pub(super) struct Parser {
    pub expression: String,
    pub label: String,
    /// A user parser determines its own collection shape when entries are built.
    pub repeated: Option<bool>,
}

impl Cli<'_, '_> {
    pub(super) fn field_parser(&mut self, ty: &Value, origin: &SourceLocation) -> Result<Parser> {
        let interface = application(&self.parser, ty);
        let mut injected = None;
        for field in &self.dependencies {
            if self.ctx.subtype(&field.ty.applied_type, &interface)? {
                if injected.is_some() {
                    return Err(Error::at(
                        origin,
                        "ambiguous primary val dependencies for a CLI field parser",
                    ));
                }
                injected = Some(format!("this.{}", ident(&field.name)));
            }
        }
        let info = self.ctx.describe(ty)?;
        let label = info["name"].as_str().unwrap_or("").to_owned();
        if let Some(expression) = injected {
            return Ok(Parser {
                expression,
                label,
                repeated: None,
            });
        }
        let parsers = self.name("CliParsers");
        for scalar in [
            "String", "Char", "Boolean", "Int8", "Int16", "Int", "Long", "UInt8", "UInt16", "UInt",
            "ULong", "Float", "Double",
        ] {
            if ty["declaration"] == self.ctx.core_id(scalar)? {
                let first = scalar[..1].to_ascii_lowercase();
                return Ok(Parser {
                    expression: format!("{parsers}.{first}{}Value", &scalar[1..]),
                    label: scalar.into(),
                    repeated: Some(false),
                });
            }
        }
        if let [element] = types(ty) {
            for (owner, factory) in [
                ("Option", "optional"),
                ("Array", "array"),
                ("MutableArray", "mutableArray"),
                ("ArrayList", "arrayList"),
            ] {
                if ty["declaration"] != self.ctx.core_id(owner)? {
                    continue;
                }
                let child = self.field_parser(element, origin)?;
                if owner != "Option" && child.repeated == Some(true) {
                    return Err(Error::at(
                        origin,
                        "nested sequences require an explicit CLI value parser",
                    ));
                }
                return Ok(Parser {
                    expression: format!("{parsers}.{factory}({})", child.expression),
                    label: child.label,
                    repeated: if owner == "Option" {
                        child.repeated
                    } else {
                        Some(true)
                    },
                });
            }
        }
        let kind = &info["shape"]["nominalKind"];
        if kind["kind"] == "enum" && kind["integral"] != true {
            let members = self.ctx.members(ty)?;
            if !members.variants.is_empty() && members.variants.iter().all(|v| v.style == "unit") {
                let target = self.ctx.spelling(ty)?;
                let adapter = self.name("CliFieldParser");
                let shape = self.name("CliValueShape");
                let names = members
                    .variants
                    .iter()
                    .map(|v| v.name.clone())
                    .collect::<Vec<_>>();
                let arms = names
                    .iter()
                    .map(|n| format!("case {} -> {target}.{}", literal(n), ident(n)))
                    .collect::<Vec<_>>()
                    .join("\n");
                let expression = format!(
                    "{adapter}<{target}>({shape}.Choice({})) {{ cliField -> when (cliField.readString()) {{\n{arms}\nelse -> cliField.fail(\"unknown choice\")\n}} }}",
                    strings(&names)
                );
                return Ok(Parser {
                    expression,
                    label: String::new(),
                    repeated: Some(false),
                });
            }
        }
        if !info["visibleCompanion"].is_null()
            && self.ctx.subtype(&info["visibleCompanion"], &interface)?
        {
            return Ok(Parser {
                expression: format!("{}.Companion", self.ctx.spelling(ty)?),
                label,
                repeated: None,
            });
        }
        Err(Error::at(
            origin,
            format!(
                "no CLI value parser for {label}; provide a CliValueParser dependency or a conforming companion"
            ),
        ))
    }
}
