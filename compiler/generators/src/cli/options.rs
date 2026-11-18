use super::*;
use crate::model::Annotation;
use scoop_generator_protocol::SourceLocation;

#[derive(Clone, Copy)]
pub(super) enum Role {
    Option,
    Flag,
    Argument(i64),
}
pub(super) struct Options {
    pub role: Role,
    pub ignore: bool,
    pub required: bool,
    pub names: Vec<String>,
    pub negative: Vec<String>,
    pub help: String,
    pub label: String,
    pub default_text: String,
}

impl Cli<'_, '_> {
    pub(super) fn options(&mut self, slot: &Slot) -> Result<Options> {
        let boolean = slot.ty.applied_type["declaration"] == self.ctx.core_id("Boolean")?;
        let mut result = Options {
            role: if boolean { Role::Flag } else { Role::Option },
            ignore: false,
            required: false,
            names: vec![],
            negative: vec![],
            help: String::new(),
            label: String::new(),
            default_text: String::new(),
        };
        let mut seen = false;
        for name in ["CliIgnore", "CliOption", "CliFlag", "CliArgument"] {
            let id = self.export(name)?;
            let Some(annotation) = slot.annotations.iter().find(|a| a.annotation == id) else {
                continue;
            };
            if seen {
                return Err(Error::at(
                    &annotation.origin,
                    format!("conflicting CLI annotations on `{}`", slot.name),
                ));
            }
            seen = true;
            let scalar = |i: usize| &annotation.arguments[i]["value"]["value"];
            let string = |i: usize| serde_json::from_value(scalar(i).clone());
            match name {
                "CliIgnore" => result.ignore = true,
                "CliOption" => {
                    result.role = Role::Option;
                    result.names = names(annotation, 0)?;
                    result.help = string(1)?;
                    result.label = string(2)?;
                    result.default_text = string(3)?;
                    result.required = serde_json::from_value(scalar(4).clone())?;
                }
                "CliFlag" => {
                    if !boolean {
                        return Err(Error::at(
                            &annotation.origin,
                            "CliFlag requires exact Boolean",
                        ));
                    }
                    result.role = Role::Flag;
                    result.names = names(annotation, 0)?;
                    result.negative = names(annotation, 1)?;
                    result.help = string(2)?;
                    result.default_text = string(3)?;
                    result.required = serde_json::from_value(scalar(4).clone())?;
                }
                _ => {
                    let index =
                        scalar(0)
                            .as_str()
                            .and_then(|v| v.parse().ok())
                            .ok_or_else(|| {
                                Error::at(
                                    &annotation.origin,
                                    "CliArgument requires a Long position",
                                )
                            })?;
                    result.role = Role::Argument(index);
                    result.help = string(1)?;
                    result.label = string(2)?;
                    result.default_text = string(3)?;
                    result.required = serde_json::from_value(scalar(4).clone())?;
                }
            }
        }
        if result.names.is_empty() && !matches!(result.role, Role::Argument(_)) {
            result.names.push(format!("--{}", slot.name))
        }
        Ok(result)
    }
}

fn names(annotation: &Annotation, index: usize) -> Result<Vec<String>> {
    annotation.arguments[index]["elements"]
        .as_array()
        .ok_or_else(|| Error::at(&annotation.origin, "expected an array of CLI names"))?
        .iter()
        .map(|v| {
            Ok(serde_json::from_value(
                v["value"]["value"]["value"].clone(),
            )?)
        })
        .collect()
}

pub(super) fn valid_name(s: &str) -> bool {
    if let Some(long) = s.strip_prefix("--") {
        return !long.is_empty()
            && long
                .chars()
                .all(|c| c != '=' && !c.is_whitespace() && !c.is_control());
    }
    let bytes = s.as_bytes();
    bytes.len() == 2 && bytes[0] == b'-' && bytes[1].is_ascii_alphabetic()
}

pub(super) fn validate_positions(
    mut positions: Vec<(i64, bool, Option<bool>, SourceLocation)>,
) -> Result<()> {
    positions.sort_by_key(|p| p.0);
    let mut optional = false;
    for (i, (position, required, repeated, origin)) in positions.iter().enumerate() {
        let error = if *position != i as i64 {
            Some("positional indices must start at zero and be contiguous")
        } else if *repeated == Some(true) && i + 1 != positions.len() {
            Some("variadic positional must be last")
        } else if optional && *required && *repeated == Some(false) {
            Some("required positional follows an optional argument")
        } else {
            None
        };
        if let Some(error) = error {
            return Err(Error::at(origin, error));
        }
        optional |= !*required;
    }
    Ok(())
}
