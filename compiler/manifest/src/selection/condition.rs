//! Portable target conditions retained in package declarations and registry metadata.

use std::collections::{BTreeMap, BTreeSet};

use super::SourceTarget;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, serde::Serialize)]
#[serde(rename_all = "lowercase")]
enum Axis {
    Arch,
    Os,
    Env,
}

impl Axis {
    fn value<'a>(self, target: &SourceTarget<'a>) -> &'a str {
        match self {
            Self::Arch => target.arch,
            Self::Os => target.os,
            Self::Env => target.env,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(try_from = "BTreeMap<String, toml::Value>")]
pub struct TargetCondition(BTreeMap<Axis, BTreeSet<String>>);

impl TargetCondition {
    pub fn matches(&self, target: &SourceTarget<'_>) -> bool {
        self.0
            .iter()
            .all(|(axis, values)| values.contains(axis.value(target)))
    }

    pub fn validate(&self, catalog: &[SourceTarget<'_>]) -> Result<(), String> {
        for (&axis, values) in &self.0 {
            validate_values(axis, values, catalog)?;
        }
        Ok(())
    }

    pub(super) fn insert(
        &mut self,
        key: String,
        raw: toml::Value,
        catalog: Option<&[SourceTarget<'_>]>,
    ) -> Result<(), String> {
        let axis = match key.as_str() {
            "arch" => Axis::Arch,
            "os" => Axis::Os,
            "env" => Axis::Env,
            _ => return Err(format!("unknown target predicate key {key:?}")),
        };
        let values = match raw {
            toml::Value::String(value) => BTreeSet::from([value]),
            toml::Value::Array(items) => items
                .into_iter()
                .map(|item| match item {
                    toml::Value::String(value) => Ok(value),
                    _ => Err(format!("target predicate {key:?} requires strings")),
                })
                .collect::<Result<_, _>>()?,
            _ => {
                return Err(format!(
                    "target predicate {key:?} requires a string or an array of strings"
                ));
            }
        };
        if let Some(catalog) = catalog {
            validate_values(axis, &values, catalog)?;
        }
        self.0.insert(axis, values);
        Ok(())
    }
}

fn validate_values(
    axis: Axis,
    values: &BTreeSet<String>,
    catalog: &[SourceTarget<'_>],
) -> Result<(), String> {
    for value in values {
        if !catalog.iter().any(|target| axis.value(target) == value) {
            let key = match axis {
                Axis::Arch => "arch",
                Axis::Os => "os",
                Axis::Env => "env",
            };
            return Err(format!("unknown target predicate {key:?} value {value:?}"));
        }
    }
    Ok(())
}

impl TryFrom<BTreeMap<String, toml::Value>> for TargetCondition {
    type Error = String;

    fn try_from(fields: BTreeMap<String, toml::Value>) -> Result<Self, String> {
        let mut condition = Self::default();
        for (name, value) in fields {
            condition.insert(name, value, None)?;
        }
        Ok(condition)
    }
}
