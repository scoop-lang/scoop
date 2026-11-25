//! Bind already expanded declarations to the selected exact graph.

use super::*;
use crate::package::{DependencyDeclaration, InheritableValue, PackageDocument};

impl ParsedConeManifest {
    pub fn into_compiler_projection(mut self) -> Self {
        self.semantic.native = self.semantic.native.compiler_projection();
        self
    }

    /// A compiler handoff uses bound coordinates; discovery locators are no longer inputs.
    pub fn into_bound(mut self) -> Self {
        for locator in self.locators.0.values_mut() {
            *locator = DependencyLocator::SearchRoots;
        }
        for dependency in self.tests.dependencies.values_mut() {
            dependency.locator = DependencyLocator::SearchRoots;
        }
        self
    }

    /// The caller has expanded the coordinate and selected every dependency.
    /// This operation neither resolves versions nor interprets source locators.
    pub fn bind_package(
        document: &PackageDocument,
        coordinate: ConeCoordinate,
        production: &[ConeCoordinate],
        test: Option<&[ConeCoordinate]>,
    ) -> Result<Self, ManifestParseError> {
        let cone = document.cone.as_ref().ok_or_else(|| {
            ManifestParseError::new(
                ManifestParseErrorKind::InvalidPackage(
                    scoop_source::DiagnosticCode::E2009,
                    "a virtual workspace is not a Cone".into(),
                ),
                Some(document.schema_span.clone()),
            )
        })?;
        let declared = bind(&document.dependencies, production)?;
        let tests = if let Some(test) = test {
            TestManifestConfig {
                isolate: document.isolate_tests,
                sources: document.test_sources.clone(),
                dependencies: bind(&document.test_dependencies, test)?,
            }
        } else {
            TestManifestConfig {
                isolate: false,
                sources: SourceSelection::Explicit(Vec::new()),
                dependencies: BTreeMap::new(),
            }
        };
        let semantic = ConeManifestSemantic {
            coordinate,
            requested_kind: cone.kind,
            dependencies: declared
                .iter()
                .map(|(key, value)| (key.clone(), value.coordinate.clone()))
                .collect(),
            sources: document.sources.clone(),
            native: document.native.clone(),
            windows: document.windows.clone(),
        };
        let locators = DependencyLocatorTable(
            declared
                .keys()
                .map(|key| (key.clone(), DependencyLocator::SearchRoots))
                .collect(),
        );
        Ok(Self {
            semantic,
            locators,
            diagnostic_spans: ManifestDiagnosticSpans {
                schema: ManifestSpan::new(document.schema_span.clone()),
                cone: ManifestSpan::new(cone.span.clone()),
                cone_fields: ConeManifestSpans {
                    group: field_span(&cone.group),
                    name: ManifestSpan::new(cone.name.span.clone()),
                    version: field_span(&cone.version),
                    kind: ManifestSpan::new(cone.kind_span.clone()),
                },
                dependencies: declared
                    .into_iter()
                    .map(|(key, value)| (key, value.spans))
                    .collect(),
            },
            tests,
        })
    }
}

fn field_span(value: &InheritableValue) -> ManifestSpan {
    ManifestSpan::new(match value {
        InheritableValue::Value(value) => value.span.clone(),
        InheritableValue::Workspace { span } => span.clone(),
    })
}

fn bind(
    declarations: &BTreeMap<DependencyCoordinateKey, DependencyDeclaration>,
    bindings: &[ConeCoordinate],
) -> Result<BTreeMap<DependencyCoordinateKey, ManifestDependency>, ManifestParseError> {
    let mut result = BTreeMap::new();
    for coordinate in bindings {
        let key = DependencyCoordinateKey::from_coordinate(coordinate);
        let Some(declaration) = declarations.get(&key) else {
            if *coordinate == ConeCoordinate::reserved_core() {
                continue;
            }
            return Err(ManifestParseError::new(
                ManifestParseErrorKind::InvalidPackage(
                    scoop_source::DiagnosticCode::ICE2201,
                    format!("no declaration for bound dependency {key}"),
                ),
                None,
            ));
        };
        let span = ManifestSpan::new(declaration.span().clone());
        result.insert(
            key,
            ManifestDependency {
                coordinate: coordinate.clone(),
                locator: DependencyLocator::SearchRoots,
                spans: DependencyManifestSpans {
                    declaration: span.clone(),
                    version: span,
                    path: None,
                    artifact: None,
                },
            },
        );
    }
    Ok(result)
}
