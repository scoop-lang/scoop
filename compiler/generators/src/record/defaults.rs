use super::*;
use serde_json::json;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SimpleDefault {
    Boolean(bool),
    None,
    EmptyArray,
}

impl SimpleDefault {
    pub fn source(self, option: &str) -> String {
        match self {
            Self::Boolean(value) => value.to_string(),
            Self::None => format!("{option}.None"),
            Self::EmptyArray => "[]".into(),
        }
    }
}

pub(super) fn read(
    ctx: &mut Context<'_>,
    slot: &Slot,
    option: &Value,
) -> Result<Option<SimpleDefault>> {
    let Some(reference) = &slot.default else {
        return Ok(None);
    };
    let response = ctx.query(json!({
        "kind": "describeDefaults",
        "defaults": [{"owner": reference["owner"], "parameter": reference["parameter"]}]
    }))?;
    Ok(classify(
        &response["defaults"][0]["definition"]["body"],
        slot.ty.declaration_type["declaration"] == *option,
    ))
}

fn classify(body: &Value, declared_option: bool) -> Option<SimpleDefault> {
    if !body["statements"].as_array().is_some_and(Vec::is_empty) {
        return None;
    }
    let op = &body["value"]["operation"];
    match op["kind"].as_str()? {
        "booleanLiteral" => op["value"].as_bool().map(SimpleDefault::Boolean),
        "noneLiteral" if declared_option => Some(SimpleDefault::None),
        "variantConstruct"
            if declared_option && op["arguments"].as_array().is_some_and(Vec::is_empty) =>
        {
            Some(SimpleDefault::None)
        }
        "arrayLiteral" if op["elements"].as_array().is_some_and(Vec::is_empty) => {
            Some(SimpleDefault::EmptyArray)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests;
