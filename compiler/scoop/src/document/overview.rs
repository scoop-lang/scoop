use super::*;
use pulldown_cmark::{Event, Parser, Tag};

#[cfg(test)]
mod tests;

pub(super) struct Overview {
    pub key: String,
    pub view: scoop_doc::ConeOverview,
    pub copies: Vec<(PathBuf, String)>,
    pub warnings: Vec<StructuredDiagnosticV1>,
}

struct ParsedReadme {
    text: String,
    references: Vec<Reference>,
}

struct Reference {
    url: String,
    start: usize,
    end: usize,
}

pub(super) fn select(source: &plan::Source) -> BuildResult<Overview> {
    let path = source
        .readme
        .clone()
        .unwrap_or_else(|| source.root.join("README.md"));
    let state = plan::file_state(&path)?;
    let key = input_key(&(path.as_os_str().as_encoded_bytes(), &state))?;
    let mut overview = Overview {
        key,
        view: scoop_doc::ConeOverview {
            readme: None,
            dependencies: source.dependencies.clone(),
        },
        copies: Vec::new(),
        warnings: Vec::new(),
    };
    if state.is_none() {
        if source.readme.is_some() {
            return Err(io_error(&path, "declared readme does not exist"));
        }
        return Ok(overview);
    }
    let parsed = parsed(&path)?;
    let mut states = Vec::new();
    let mut resources = BTreeMap::new();
    let cone = scoop_doc::ConeKey::new(source.identity).map_err(error)?;
    for reference in parsed.references {
        let url = &reference.url;
        if url.starts_with(['#', '/'])
            || url
                .split(['/', '?', '#'])
                .next()
                .unwrap_or("")
                .contains(':')
        {
            continue;
        }
        let (relative, suffix) = url
            .find(['?', '#'])
            .map_or((url.as_str(), ""), |i| (&url[..i], &url[i..]));
        if relative.is_empty() {
            continue;
        }
        let input = path
            .parent()
            .unwrap_or(&source.root)
            .join(decode_path(relative).map_err(|e| io_error(&path, e))?);
        let state = plan::file_state(&input)?;
        states.push((url.clone(), state.clone()));
        let destination = if state.is_some() {
            let extension = input
                .extension()
                .and_then(|value| value.to_str())
                .filter(|value| value.bytes().all(|byte| byte.is_ascii_alphanumeric()))
                .unwrap_or("bin");
            let name = input_key(&input.as_os_str().as_encoded_bytes())?;
            let destination = format!("c/{}/resources/{name}.{extension}", cone.0);
            overview.copies.push((input, destination.clone()));
            Some(format!("{destination}{suffix}"))
        } else {
            overview.warnings.push(
                StructuredDiagnosticV1::new(
                    scoop_protocol::DiagnosticSeverityV1::Warning,
                    "SCOOP_DOC_RESOURCE".into(),
                    format!("README resource {url:?} does not exist"),
                    scoop_protocol::DiagnosticOriginV1::HostPathSpan {
                        path: scoop_protocol::HostPathCarrier::from_path(&path).map_err(error)?,
                        span: scoop_protocol::ProtocolByteSpan::new(
                            reference.start as u64,
                            reference.end as u64,
                        )
                        .map_err(error)?,
                    },
                    Vec::new(),
                )
                .map_err(error)?,
            );
            None
        };
        resources.insert(reference.url, destination);
    }
    overview.key = input_key(&(&overview.key, states))?;
    overview.view.readme = Some(scoop_doc::ReadmeInput {
        text: parsed.text,
        resources,
    });
    Ok(overview)
}

fn parsed(path: &Path) -> BuildResult<ParsedReadme> {
    let text = std::fs::read_to_string(path).map_err(|e| io_error(path, e))?;
    let mut references = BTreeMap::new();
    for (event, range) in Parser::new(&text).into_offset_iter() {
        if let Event::Start(Tag::Link { dest_url, .. } | Tag::Image { dest_url, .. }) = event {
            let url = dest_url.to_string();
            references.entry(url.clone()).or_insert(Reference {
                url,
                start: range.start,
                end: range.end,
            });
        }
    }
    Ok(ParsedReadme {
        text,
        references: references.into_values().collect(),
    })
}

fn decode_path(path: &str) -> Result<String, &'static str> {
    let mut bytes = Vec::new();
    let mut input = path.bytes();
    while let Some(byte) = input.next() {
        if byte != b'%' {
            bytes.push(byte);
            continue;
        }
        let hi = input.next().and_then(|c| (c as char).to_digit(16));
        let lo = input.next().and_then(|c| (c as char).to_digit(16));
        let (Some(hi), Some(lo)) = (hi, lo) else {
            return Err("invalid percent encoding in README resource URL");
        };
        bytes.push((hi * 16 + lo) as u8);
    }
    String::from_utf8(bytes).map_err(|_| "README resource path is not UTF-8")
}
