use serde::Deserialize;
use toml::Spanned;

use super::*;

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Details {
    version: Option<String>,
    workspace: Option<bool>,
    path: Option<String>,
    artifact: Option<String>,
    registry: Option<String>,
    git: Option<String>,
    github: Option<String>,
    branch: Option<String>,
    tag: Option<String>,
    rev: Option<String>,
    subdir: Option<String>,
}

pub(super) fn declarations(
    values: raw::DependencyMap,
    catalog: &[crate::SourceTarget<'_>],
) -> Result<BTreeMap<DependencyCoordinateKey, DependencyDeclaration>, ManifestParseError> {
    values
        .into_iter()
        .map(|(key, value)| {
            let key = key.parse().map_err(|message| {
                invalid(scoop_source::DiagnosticCode::E2010, message, value.span())
            })?;
            let span = value.span();
            let mut raw = value.into_inner();
            let when = raw
                .as_table_mut()
                .and_then(|table| table.remove("when"))
                .map(|value| {
                    let condition: crate::TargetCondition =
                        value.try_into().map_err(|error: toml::de::Error| {
                            invalid(
                                scoop_source::DiagnosticCode::E2013,
                                error.to_string(),
                                span.clone(),
                            )
                        })?;
                    condition.validate(catalog).map_err(|message| {
                        invalid(scoop_source::DiagnosticCode::E2013, message, span.clone())
                    })?;
                    Ok::<_, ManifestParseError>(condition)
                })
                .transpose()?;
            let value = declaration(Spanned::new(span, raw), false)?;
            Ok((key, DependencyDeclaration { when, value }))
        })
        .collect()
}

pub(super) fn requests(
    values: raw::DependencyMap,
    overrides: bool,
) -> Result<BTreeMap<DependencyCoordinateKey, DependencyRequest>, ManifestParseError> {
    values
        .into_iter()
        .map(|(key, value)| {
            let key = key.parse().map_err(|message| {
                invalid(scoop_source::DiagnosticCode::E2010, message, value.span())
            })?;
            match declaration(value, overrides)? {
                DependencyValue::Request(value) => Ok((key, value)),
                DependencyValue::Workspace { span } => Err(invalid(
                    scoop_source::DiagnosticCode::E2014,
                    "workspace dependencies and overrides must declare a value, not inherit one",
                    span,
                )),
            }
        })
        .collect()
}

fn declaration(
    raw: Spanned<toml::Value>,
    overrides: bool,
) -> Result<DependencyValue, ManifestParseError> {
    let span = raw.span();
    let details = match raw.into_inner() {
        toml::Value::String(value) => Details {
            version: Some(value),
            ..Details::default()
        },
        value @ toml::Value::Table(_) => value.try_into::<Details>().map_err(|error| {
            invalid(
                scoop_source::DiagnosticCode::E2014,
                error.to_string(),
                span.clone(),
            )
        })?,
        _ => {
            return Err(invalid(
                scoop_source::DiagnosticCode::E2014,
                "dependency requires a version string or a table",
                span,
            ));
        }
    };
    let error = |code, message| invalid(code, message, span.clone());
    if let Some(workspace) = details.workspace {
        let only_workspace = details.version.is_none()
            && details.path.is_none()
            && details.artifact.is_none()
            && details.registry.is_none()
            && details.git.is_none()
            && details.github.is_none()
            && details.branch.is_none()
            && details.tag.is_none()
            && details.rev.is_none()
            && details.subdir.is_none();
        if !workspace || !only_workspace {
            return Err(error(
                scoop_source::DiagnosticCode::E2014,
                "workspace = true must be the entire dependency declaration".into(),
            ));
        }
        return Ok(DependencyValue::Workspace { span });
    }
    let source_count = [
        &details.path,
        &details.artifact,
        &details.registry,
        &details.git,
        &details.github,
    ]
    .into_iter()
    .filter(|value| value.is_some())
    .count();
    if source_count > 1 {
        return Err(error(
            scoop_source::DiagnosticCode::E2014,
            "a dependency must select only one source".into(),
        ));
    }
    if overrides && source_count == 0 {
        return Err(error(
            scoop_source::DiagnosticCode::E2014,
            "an override must specify a source".into(),
        ));
    }
    let requirement = details
        .version
        .as_deref()
        .map(str::parse::<VersionRequirement>)
        .transpose()
        .map_err(|message| error(scoop_source::DiagnosticCode::E2015, message))?;
    if details.git.is_none()
        && details.github.is_none()
        && [&details.branch, &details.tag, &details.rev, &details.subdir]
            .iter()
            .any(|value| value.is_some())
    {
        return Err(error(
            scoop_source::DiagnosticCode::E2014,
            "branch, tag, rev, and subdir require a Git source".into(),
        ));
    }
    let source = if let Some(value) = &details.path {
        PackageSource::Path(
            path(value)
                .map_err(|message| error(scoop_source::DiagnosticCode::E2012, message.into()))?,
        )
    } else if let Some(value) = &details.artifact {
        if requirement
            .as_ref()
            .and_then(VersionRequirement::exact_version)
            .is_none()
        {
            return Err(error(
                scoop_source::DiagnosticCode::E2014,
                "artifact requires one exact =version".into(),
            ));
        }
        PackageSource::Artifact(
            path(value)
                .map_err(|message| error(scoop_source::DiagnosticCode::E2012, message.into()))?,
        )
    } else if let Some(value) = &details.registry {
        PackageSource::Registry(
            value
                .parse()
                .map_err(|message| error(scoop_source::DiagnosticCode::E2016, message))?,
        )
    } else if details.git.is_some() || details.github.is_some() {
        PackageSource::Git(git(details).map_err(|(code, message)| error(code, message))?)
    } else {
        PackageSource::Default
    };
    if !overrides
        && requirement.is_none()
        && matches!(source, PackageSource::Default | PackageSource::Registry(_))
    {
        return Err(error(
            scoop_source::DiagnosticCode::E2014,
            "registry and system dependencies require a version requirement".into(),
        ));
    }
    Ok(DependencyValue::Request(DependencyRequest {
        requirement,
        source,
        span,
    }))
}

fn git(details: Details) -> Result<GitSource, (scoop_source::DiagnosticCode, String)> {
    let repository = if let Some(repository) = details.github {
        let parts: Vec<_> = repository.split('/').collect();
        if parts.len() != 2
            || parts.iter().any(|part| {
                part.is_empty()
                    || *part == "."
                    || *part == ".."
                    || part
                        .chars()
                        .any(|c| c.is_whitespace() || c == '\0' || c == ':')
            })
        {
            return Err((
                scoop_source::DiagnosticCode::E2016,
                "github requires owner/repo".into(),
            ));
        }
        format!("https://github.com/{repository}.git")
    } else {
        details.git.expect("Git source was selected")
    };
    if repository.is_empty() || repository.contains('\0') {
        return Err((
            scoop_source::DiagnosticCode::E2016,
            "Git repository must not be empty or contain NUL".into(),
        ));
    }
    let mut references = Vec::new();
    for (value, constructor) in [
        (
            details.branch,
            GitReference::Branch as fn(String) -> GitReference,
        ),
        (details.tag, GitReference::Tag),
        (details.rev, GitReference::Revision),
    ] {
        if let Some(value) = value {
            if value.is_empty() || value.contains('\0') {
                return Err((
                    scoop_source::DiagnosticCode::E2014,
                    "Git reference must not be empty or contain NUL".into(),
                ));
            }
            references.push(constructor(value));
        }
    }
    if references.len() > 1 {
        return Err((
            scoop_source::DiagnosticCode::E2014,
            "Git dependency must choose at most one of branch, tag, and rev".into(),
        ));
    }
    let subdir = ConeRelativePath::new(details.subdir.as_deref().unwrap_or("."))
        .map_err(|message| (scoop_source::DiagnosticCode::E2012, message.to_owned()))?;
    Ok(GitSource {
        repository,
        reference: references.pop().unwrap_or(GitReference::DefaultBranch),
        subdir,
    })
}
