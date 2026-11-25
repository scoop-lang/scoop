use std::collections::BTreeMap;

use serde::Deserialize;
use toml::Spanned;

use super::*;
use crate::{ManifestParseError, ManifestParseErrorKind};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawConditionalPath {
    path: Spanned<String>,
    when: Option<BTreeMap<String, Spanned<toml::Value>>>,
}

pub(crate) fn parse_sources(
    raw: Option<Vec<RawConditionalPath>>,
    catalog: &[SourceTarget<'_>],
) -> Result<SourceSelection, ManifestParseError> {
    match raw {
        None => Ok(SourceSelection::Default),
        Some(raw) => raw
            .into_iter()
            .map(|raw| parse_path(raw, catalog, false))
            .collect::<Result<Vec<_>, _>>()
            .map(SourceSelection::Explicit),
    }
}

pub(crate) fn parse_native_path(
    raw: RawConditionalPath,
    catalog: &[SourceTarget<'_>],
) -> Result<ConditionalSourcePath, ManifestParseError> {
    parse_path(raw, catalog, true)
}

fn parse_path(
    raw: RawConditionalPath,
    catalog: &[SourceTarget<'_>],
    native: bool,
) -> Result<ConditionalSourcePath, ManifestParseError> {
    let path = if native {
        ConeRelativePath::native(raw.path.get_ref())
    } else {
        ConeRelativePath::new(raw.path.get_ref())
    }
    .map_err(|reason| {
        ManifestParseError::new(
            ManifestParseErrorKind::InvalidSelection(
                scoop_source::DiagnosticCode::E2012,
                format!("invalid source path {:?}: {reason}", raw.path.get_ref()),
            ),
            Some(raw.path.span()),
        )
    })?;
    Ok(ConditionalSourcePath {
        path,
        predicate: parse_predicate(raw.when, catalog)?,
    })
}

pub(crate) fn parse_predicate(
    when: Option<BTreeMap<String, Spanned<toml::Value>>>,
    catalog: &[SourceTarget<'_>],
) -> Result<TargetPredicate, ManifestParseError> {
    let mut condition = TargetCondition::default();
    for (key, value) in when.into_iter().flatten() {
        let span = value.span();
        condition
            .insert(key, value.into_inner(), Some(catalog))
            .map_err(|message| {
                ManifestParseError::new(
                    ManifestParseErrorKind::InvalidSelection(
                        scoop_source::DiagnosticCode::E2013,
                        message,
                    ),
                    Some(span),
                )
            })?;
    }
    let mut targets: Vec<_> = catalog
        .iter()
        .filter(|target| condition.matches(target))
        .collect();
    targets.sort_by(|a, b| a.triple.cmp(b.triple));
    Ok(TargetPredicate {
        targets: targets
            .into_iter()
            .map(|target| SelectedSourceTarget {
                id: target.id,
                triple: target.triple.to_owned(),
            })
            .collect(),
    })
}
