//! Swift package production declarations; parsing performs no package or tool I/O.

use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;

use scoop_identity::{CanonicalNativeLibraryName, NativeLibraryKind};
use serde::{Deserialize, Serialize};
use toml::Spanned;

use super::{NativeConfig, NativeLibrary};
use crate::{
    ConeRelativePath, ManifestParseError, ManifestParseErrorKind, SourceTarget, TargetPredicate,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SwiftLinkage {
    Static,
    Dynamic,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "format", rename_all = "snake_case", deny_unknown_fields)]
pub enum SwiftLibraryFormat {
    Library,
    Framework { bundle_id: String },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SwiftLibrary {
    package: ConeRelativePath,
    #[serde(with = "crate::serde_record::library_name")]
    product: CanonicalNativeLibraryName,
    linkage: SwiftLinkage,
    format: SwiftLibraryFormat,
    predicate: TargetPredicate,
    inputs: Vec<ConeRelativePath>,
    env: Vec<String>,
    span: Range<usize>,
}

impl SwiftLibrary {
    pub fn package(&self) -> &ConeRelativePath {
        &self.package
    }
    pub fn product(&self) -> &CanonicalNativeLibraryName {
        &self.product
    }
    pub fn linkage(&self) -> SwiftLinkage {
        self.linkage
    }
    pub fn format(&self) -> &SwiftLibraryFormat {
        &self.format
    }
    pub fn predicate(&self) -> &TargetPredicate {
        &self.predicate
    }
    pub fn inputs(&self) -> &[ConeRelativePath] {
        &self.inputs
    }
    pub fn env(&self) -> &[String] {
        &self.env
    }
    pub fn span(&self) -> Range<usize> {
        self.span.clone()
    }

    pub fn library_kind(&self) -> NativeLibraryKind {
        match (&self.format, self.linkage) {
            (SwiftLibraryFormat::Framework { .. }, _) => NativeLibraryKind::Framework,
            (SwiftLibraryFormat::Library, SwiftLinkage::Static) => NativeLibraryKind::StaticArchive,
            (SwiftLibraryFormat::Library, SwiftLinkage::Dynamic) => NativeLibraryKind::Dynamic,
        }
    }

    pub(super) fn requirement(&self) -> NativeLibrary {
        NativeLibrary {
            name: self.product.clone(),
            kind: self.library_kind(),
            predicate: self.predicate.clone(),
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawSwiftLibrary {
    package: Spanned<String>,
    product: Spanned<String>,
    linkage: Spanned<String>,
    format: Option<Spanned<String>>,
    bundle_id: Option<Spanned<String>>,
    when: Option<BTreeMap<String, Spanned<toml::Value>>>,
    #[serde(default)]
    inputs: Vec<Spanned<String>>,
    #[serde(default)]
    env: Vec<Spanned<String>>,
}

pub(super) fn parse(
    raw: Spanned<RawSwiftLibrary>,
    catalog: &[SourceTarget<'_>],
) -> Result<SwiftLibrary, ManifestParseError> {
    let span = raw.span();
    let raw = raw.into_inner();
    let package = parse_path(raw.package)?;
    let product = CanonicalNativeLibraryName::new(raw.product.get_ref())
        .map_err(|cause| error(raw.product.span(), cause))?;
    let linkage = match raw.linkage.get_ref().as_str() {
        "static" => SwiftLinkage::Static,
        "dynamic" => SwiftLinkage::Dynamic,
        _ => {
            return Err(error(
                raw.linkage.span(),
                "linkage must be static or dynamic",
            ));
        }
    };
    let format = match raw.format.as_ref().map(|value| value.get_ref().as_str()) {
        None | Some("library") => {
            if let Some(bundle) = raw.bundle_id {
                return Err(error(
                    bundle.span(),
                    "bundle_id requires format = framework",
                ));
            }
            SwiftLibraryFormat::Library
        }
        Some("framework") => {
            let bundle = raw
                .bundle_id
                .ok_or_else(|| error(span.clone(), "framework requires bundle_id"))?;
            if bundle.get_ref().split('.').any(str::is_empty)
                || !bundle
                    .get_ref()
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || b".-".contains(&byte))
            {
                return Err(error(bundle.span(), "invalid framework bundle_id"));
            }
            SwiftLibraryFormat::Framework {
                bundle_id: bundle.into_inner(),
            }
        }
        Some(_) => {
            return Err(error(
                raw.format.expect("explicit format").span(),
                "format must be library or framework",
            ));
        }
    };
    let mut env = BTreeSet::new();
    for variable in raw.env {
        if variable.get_ref().is_empty() || variable.get_ref().contains(['=', '\0']) {
            return Err(error(variable.span(), "invalid environment variable name"));
        }
        if !env.insert(variable.get_ref().clone()) {
            return Err(error(variable.span(), "duplicate environment variable"));
        }
    }
    Ok(SwiftLibrary {
        package,
        product,
        linkage,
        format,
        predicate: crate::selection::parse_predicate(raw.when, catalog)?,
        inputs: raw
            .inputs
            .into_iter()
            .map(parse_path)
            .collect::<Result<_, _>>()?,
        env: env.into_iter().collect(),
        span,
    })
}

pub(super) fn validate(
    config: &NativeConfig,
    catalog: &[SourceTarget<'_>],
) -> Result<(), ManifestParseError> {
    for target in catalog {
        let mut selected = BTreeSet::new();
        for swift in config.swift_libraries(target.id) {
            if !selected.insert(swift.product()) {
                return Err(error(
                    swift.span(),
                    format!(
                        "duplicate Swift product {:?} for {}",
                        swift.product().as_str(),
                        target.triple
                    ),
                ));
            }
            if config.libraries.iter().any(|library| {
                library.predicate.matches(target.id)
                    && &library.name == swift.product()
                    && library.kind != swift.library_kind()
            }) {
                return Err(error(
                    swift.span(),
                    format!(
                        "Swift product {:?} conflicts with native.libraries kind for {}",
                        swift.product().as_str(),
                        target.triple
                    ),
                ));
            }
        }
    }
    Ok(())
}

fn parse_path(raw: Spanned<String>) -> Result<ConeRelativePath, ManifestParseError> {
    ConeRelativePath::native(raw.get_ref()).map_err(|cause| error(raw.span(), cause))
}

fn error(span: Range<usize>, cause: impl std::fmt::Display) -> ManifestParseError {
    ManifestParseError::new(
        ManifestParseErrorKind::InvalidNative(
            scoop_source::DiagnosticCode::E2017,
            format!("invalid Swift library: {cause}"),
        ),
        Some(span),
    )
}

#[cfg(test)]
mod tests;
