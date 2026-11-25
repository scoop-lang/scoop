//! Checks the official test markers after ordinary annotation binding.

use scoop_ast::{Diagnostic, DiagnosticNote};
use scoop_hir as hir;
use std::collections::HashMap;

use crate::{Lowerer, Owner};

mod catalogue;
mod markers;
mod shapes;
use catalogue::PendingContainer;
use markers::TestMarker;

impl Lowerer {
    pub(crate) fn validate_test_annotation_source(
        &mut self,
        application: &hir::SourceAnnotationApplication,
    ) {
        if let Some(marker) = TestMarker::from_identity(application.annotation)
            && self.intrinsic_sources[application.definition_origin.file as usize].role
                != scoop_identity::SourceRole::Test
        {
            self.test_annotation_error(
                scoop_source::DiagnosticCode::E1703,
                application.definition_origin,
                format!("`@{}` is only allowed in test sources", marker.name(),),
            );
        }
    }

    pub(crate) fn discover_tests(&mut self) {
        let mut targets = self
            .annotation_metadata
            .targets
            .iter()
            .filter_map(|target| {
                let mut use_process = false;
                let mut ignore_reason = None;
                let markers = target
                    .annotations
                    .iter()
                    .filter_map(|application| {
                        let marker = TestMarker::from_identity(application.annotation)?;
                        match (marker, application.arguments.as_slice()) {
                            (
                                TestMarker::Test,
                                [
                                    hir::CanonicalAnnotationValueV1::Scalar(
                                        hir::CanonicalConstValueV1::Boolean(value),
                                    ),
                                ],
                            ) => use_process = (*value).into(),
                            (
                                TestMarker::Ignore,
                                [
                                    hir::CanonicalAnnotationValueV1::Scalar(
                                        hir::CanonicalConstValueV1::String(reason),
                                    ),
                                ],
                            ) => ignore_reason = Some(reason.clone()),
                            _ => {}
                        }
                        Some((marker, application.definition_origin))
                    })
                    .collect::<Vec<_>>();
                (!markers.is_empty()).then_some((
                    target.target,
                    markers,
                    use_process,
                    ignore_reason,
                ))
            })
            .collect::<Vec<_>>();
        targets.sort_by_key(|(_, markers, _, _)| (markers[0].1.file, markers[0].1.span.start));
        let mut containers = HashMap::new();
        for (target, markers, use_process, _) in &targets {
            let hir::SourceAnnotationTarget::Nominal(owner) = *target else {
                continue;
            };
            let Some((_, origin)) = markers
                .iter()
                .find(|(marker, _)| *marker == TestMarker::Test)
            else {
                continue;
            };
            match owner {
                hir::NominalOwner::Class(class) if self.valid_test_container(class) => {
                    containers.insert(class, PendingContainer::new(class, *origin, *use_process));
                }
                _ => self.test_annotation_error(scoop_source::DiagnosticCode::E1704, *origin,
                    "a test container must be a top-level non-generic concrete class; abstract, sealed, local and nested declarations are not allowed".into()),
            }
        }
        let methods = self
            .classes
            .iter()
            .flat_map(|(class, declaration)| {
                declaration
                    .methods
                    .iter()
                    .map(move |&method| (method, class))
            })
            .collect::<HashMap<_, _>>();
        let mut hooks = HashMap::new();
        for (target, markers, use_process, ignore_reason) in targets {
            let hir::SourceAnnotationTarget::Callable(hir::SourceAnnotationCallable::Function(
                function,
            )) = target
            else {
                continue;
            };
            let origin = markers[0].1;
            let roles = markers
                .iter()
                .filter(|(marker, _)| *marker != TestMarker::Ignore)
                .collect::<Vec<_>>();
            if let Some((_, ignore_origin)) = markers
                .iter()
                .find(|(marker, _)| *marker == TestMarker::Ignore)
                && !markers
                    .iter()
                    .any(|(marker, _)| *marker == TestMarker::Test)
            {
                self.test_annotation_error(
                    scoop_source::DiagnosticCode::E1705,
                    *ignore_origin,
                    "`@Ignore` requires `@Test` on the same test method".into(),
                );
            }
            if roles.len() > 1 {
                self.test_annotation_error(
                    scoop_source::DiagnosticCode::E1705,
                    origin,
                    "a method may have only one of `@Test`, `@BeforeTest` and `@AfterTest`".into(),
                );
            }
            let Some(&class) = methods
                .get(&function)
                .filter(|class| containers.contains_key(class))
            else {
                self.test_annotation_error(
                    scoop_source::DiagnosticCode::E1706,
                    origin,
                    "a test or lifecycle method must be declared directly in a `@Test` class"
                        .into(),
                );
                continue;
            };
            if let Some(reason) = self.test_method_rejection(function) {
                self.test_annotation_error(scoop_source::DiagnosticCode::E1707, origin, format!("invalid test or lifecycle method: {reason}; expected a safe synchronous instance method with no parameters or type parameters returning Unit"));
            }
            for &(marker, origin) in roles {
                let method = hir::TestMethod {
                    function,
                    name: self
                        .functions
                        .source(function)
                        .expect("source function declaration")
                        .name
                        .clone(),
                    origin,
                };
                match marker {
                    TestMarker::Test => {
                        containers.get_mut(&class).unwrap().cases.push((
                            method,
                            ignore_reason.clone(),
                            use_process,
                        ));
                    }
                    TestMarker::Before => containers.get_mut(&class).unwrap().before = Some(method),
                    TestMarker::After => containers.get_mut(&class).unwrap().after = Some(method),
                    TestMarker::Ignore => unreachable!("Ignore is not a method role"),
                }

                if matches!(marker, TestMarker::Before | TestMarker::After)
                    && let Some(previous) = hooks.insert((class, marker), origin)
                {
                    self.diagnostics.push(
                        Diagnostic::at_file(
                            scoop_source::DiagnosticCode::E1708,
                            origin.file as usize,
                            origin.span,
                            format!(
                                "a test container may declare at most one `@{}` method",
                                marker.name()
                            ),
                        )
                        .with_note(DiagnosticNote::at(
                            scoop_source::DiagnosticCode::N0001,
                            previous.file as usize,
                            previous.span,
                            "the other lifecycle method is declared here",
                        )),
                    );
                }
            }
        }
        if self.diagnostics.is_empty() {
            let mut pending = containers.into_values().collect::<Vec<_>>();
            pending.sort_by_key(|container| {
                (
                    self.intrinsic_sources[container.origin.file as usize]
                        .identity
                        .logical_path(),
                    container.origin.span.start,
                )
            });
            for container in pending {
                if let Some(container) = self.finish_test_container(container) {
                    self.test_containers.push(container);
                }
            }
        }
    }

    fn test_annotation_error(
        &mut self,
        code: scoop_source::DiagnosticCode,
        origin: hir::DefinitionOrigin,
        message: String,
    ) {
        self.diagnostics.push(Diagnostic::at_file(
            code,
            origin.file as usize,
            origin.span,
            message,
        ));
    }
}
