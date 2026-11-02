//! Requested-kind validation and deterministic root-entry selection.

use scoop_ast::{Diagnostic, DiagnosticNote, DiagnosticSeverity, Span};
use scoop_hir as hir;
use scoop_identity::{DefinitionOriginSubject, RequestedConeKind, SourceIdentity, SourceSpan};

const NATIVE_APP_FORM: &str = "`fun appInit(hostApi: Ptr<Unit>, outAppApi: Ptr<Unit>): Unit`";

const ENTRY_FORMS: &str = "`fun main(): Unit`, `fun main(): Int`, `fun main(args: Array<String>): Unit`, or `fun main(args: Array<String>): Int`";

/// Turn an unvalidated requested kind into the closed HIR output contract.
///
/// The library branch intentionally returns before inspecting declarations.
pub fn select_cone_output_kind(
    module: &hir::ExportHir,
    requested: RequestedConeKind,
) -> Result<hir::ConeOutputKind, Vec<Diagnostic>> {
    if requested == RequestedConeKind::Library {
        return Ok(hir::ConeOutputKind::Library);
    }

    let mut declarations = match current_entry_declarations(module, requested) {
        Ok(declarations) => declarations,
        Err(message) => return Err(vec![invalid_hir_diagnostic(message)]),
    };
    declarations.sort_by(|left, right| left.sort_key.cmp(&right.sort_key));

    let (kind, forms) = match requested {
        RequestedConeKind::Library => unreachable!("library returned before entry selection"),
        RequestedConeKind::Executable => ("executable", ENTRY_FORMS),
        RequestedConeKind::NativeApp => ("native-app", NATIVE_APP_FORM),
    };
    let valid = declarations
        .iter()
        .filter(|declaration| {
            requested == RequestedConeKind::NativeApp || declaration.rejections.is_empty()
        })
        .collect::<Vec<_>>();
    match valid.as_slice() {
        [declaration] if declaration.rejections.is_empty() => match requested {
            RequestedConeKind::Library => unreachable!("library returned before entry selection"),
            RequestedConeKind::Executable => {
                hir::LocalExecutableEntry::try_new(module, declaration.function)
                    .map(|local_entry| hir::ConeOutputKind::Executable {
                        local_entry: Box::new(local_entry),
                    })
                    .map_err(|error| vec![invalid_hir_diagnostic(error.to_string())])
            }
            RequestedConeKind::NativeApp => {
                hir::LocalNativeAppEntry::try_new(module, declaration.function)
                    .map(|local_entry| hir::ConeOutputKind::NativeApp {
                        local_entry: Box::new(local_entry),
                    })
                    .map_err(|error| vec![invalid_hir_diagnostic(error.to_string())])
            }
        },
        [] | [_] => Err(vec![missing_entry_diagnostic(
            module,
            &declarations,
            kind,
            forms,
        )]),
        [first, rest @ ..] => {
            let diagnostic = Diagnostic::at_file(
                first.file,
                first.span,
                format!("multiple {kind} entries: declare exactly one ordinary {forms}"),
            );
            let diagnostic = rest.iter().fold(diagnostic, |diagnostic, declaration| {
                diagnostic.with_note(DiagnosticNote::at(
                    declaration.file,
                    declaration.span,
                    format!("another {kind} entry candidate is declared here"),
                ))
            });
            Err(vec![diagnostic])
        }
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct PersistentSourceLocation {
    source: SourceIdentity,
    span: SourceSpan,
    identity_kind: SourceFunctionIdentityKind,
    identity: [u8; 32],
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum SourceFunctionIdentityKind {
    Plain,
    Generic,
}

struct EntryDeclaration {
    function: hir::FunctionId,
    file: usize,
    span: Span,
    sort_key: PersistentSourceLocation,
    rejections: Vec<&'static str>,
}

fn current_entry_declarations(
    module: &hir::ExportHir,
    requested: RequestedConeKind,
) -> Result<Vec<EntryDeclaration>, String> {
    let name = if requested == RequestedConeKind::NativeApp {
        "appInit"
    } else {
        "main"
    };
    let mut declarations = Vec::new();
    for function_id in module.top_level.iter().copied() {
        let function = &module.functions[function_id];
        if function.name != name
            || (requested == RequestedConeKind::Executable && function.method.is_some())
        {
            continue;
        }
        let Some(identity) = module.function_identities[function_id].source_identity() else {
            continue;
        };
        if identity.declaration().origin() != module.cone
            || !identity.declaration().owners().owners().is_empty()
        {
            continue;
        }

        let (subject, identity_kind, identity_bytes, generic) = match identity {
            hir::HirSourceFunctionIdentity::Plain(record) => (
                DefinitionOriginSubject::Function(record.id()),
                SourceFunctionIdentityKind::Plain,
                *record.id().as_array(),
                false,
            ),
            hir::HirSourceFunctionIdentity::Generic(record) => (
                DefinitionOriginSubject::GenericFunction(record.id()),
                SourceFunctionIdentityKind::Generic,
                *record.id().as_array(),
                true,
            ),
        };
        let origin = module
            .export_definition_origins
            .get(subject)
            .ok_or_else(|| "a current entry declaration has no definition origin".to_string())?
            .origin();
        if origin.source().cone() != module.cone {
            return Err(
                "a current entry declaration has a cross-Cone definition origin".to_string(),
            );
        }
        let file = module
            .source_files
            .iter()
            .position(|source| &source.identity == origin.source())
            .ok_or_else(|| {
                "a current entry definition origin is absent from the HIR source table".to_string()
            })?;
        let span = diagnostic_span(origin.span())?;
        let mut rejections = Vec::new();
        if generic || !matches!(function.genericity, hir::FunctionGenericity::Plain) {
            rejections.push("it is generic");
        }
        if function.is_suspend {
            rejections.push("it is suspend");
        }
        let ordinary_body = matches!(function.kind, hir::FunctionKind::User(_));
        let valid_parameters = if requested == RequestedConeKind::NativeApp && !ordinary_body {
            true
        } else if requested == RequestedConeKind::NativeApp {
            function.params.len() == 2 && function.params.iter().all(|parameter| matches!(module.types[parameter.ty], hir::Type::Ptr(pointee) if pointee == module.unit))
        } else {
            match function.params.as_slice() {
                [] => true,
                [parameter] => match module.types[parameter.ty] {
                    hir::Type::Class(application) => matches!(
                        module.class_applications[application].representation,
                        hir::ClassApplicationRepresentation::Intrinsic(
                            hir::IntrinsicTypeRepresentation::Array { element }
                        ) if element == module.string
                    ),
                    _ => false,
                },
                _ => false,
            }
        };
        if !valid_parameters {
            rejections.push(if requested == RequestedConeKind::NativeApp {
                "its parameters must be two ordinary `Ptr<Unit>` values"
            } else {
                "its parameters must be empty or one `Array<String>`"
            });
        }
        if module.source_parameter_interfaces.iter().any(|interface| {
            interface.owner == hir::ExportParameterOwner::Function(function_id)
                && interface.parameters.iter().any(|parameter| {
                    matches!(
                        parameter.calling,
                        hir::ExportParameterCalling::Vararg { .. }
                    )
                })
        }) {
            rejections.push("it has a vararg parameter");
        }
        if function.return_ty != module.unit
            && (requested == RequestedConeKind::NativeApp
                || !matches!(
                    module.types[function.return_ty],
                    hir::Type::Integer(hir::IntegerKind::SIGNED_32)
                ))
        {
            rejections.push(if requested == RequestedConeKind::NativeApp {
                "it must return `Unit`"
            } else {
                "it must return `Unit` or `Int`"
            });
        }
        if requested == RequestedConeKind::NativeApp {
            if function.method.is_some()
                || matches!(identity.declaration().duplicate_signature(),
                    scoop_identity::DuplicateSignatureKey::Function { receiver, .. } if receiver.is_present())
            {
                rejections.push("it has a receiver");
            }
            if !matches!(
                function.access.declared,
                hir::DeclaredVisibility::Internal | hir::DeclaredVisibility::Public
            ) {
                rejections.push("it must be internal or public");
            }
        }
        if !ordinary_body {
            rejections.push("it has no ordinary Scoop body");
        }
        declarations.push(EntryDeclaration {
            function: function_id,
            file,
            span,
            sort_key: PersistentSourceLocation {
                source: origin.source().clone(),
                span: origin.span(),
                identity_kind,
                identity: identity_bytes,
            },
            rejections,
        });
    }
    Ok(declarations)
}

fn missing_entry_diagnostic(
    module: &hir::ExportHir,
    declarations: &[EntryDeclaration],
    kind: &str,
    forms: &str,
) -> Diagnostic {
    let anchor = module
        .source_files
        .iter()
        .enumerate()
        .filter(|(_, source)| source.identity.cone() == module.cone)
        .min_by(|(_, left), (_, right)| left.identity.cmp(&right.identity));
    let diagnostic = match anchor {
        Some((file, _)) => Diagnostic::at_file(
            file,
            Span::new(0, 0),
            format!("missing {kind} entry: declare exactly one ordinary {forms}"),
        ),
        None => Diagnostic::without_span(
            DiagnosticSeverity::Error,
            0,
            format!("missing {kind} entry: current Cone has no source in HIR"),
        ),
    };
    declarations
        .iter()
        .filter(|declaration| !declaration.rejections.is_empty())
        .fold(diagnostic, |diagnostic, declaration| {
            let function = &module.functions[declaration.function];
            if kind == "native-app" && !matches!(function.kind, hir::FunctionKind::User(_)) {
                return diagnostic.with_note(DiagnosticNote::at(
                    declaration.file,
                    declaration.span,
                    format!(
                        "`{}` is not eligible because {}",
                        function.name,
                        declaration.rejections.join(", ")
                    ),
                ));
            }
            diagnostic.with_note(DiagnosticNote::at(
                declaration.file,
                declaration.span,
                format!(
                    "`fun {}({}): {}` is not eligible because {}",
                    module.functions[declaration.function].name,
                    module.functions[declaration.function]
                        .params
                        .iter()
                        .map(|parameter| format!(
                            "{}: {}",
                            parameter.name,
                            hir::type_name(module, parameter.ty)
                        ))
                        .collect::<Vec<_>>()
                        .join(", "),
                    hir::type_name(module, module.functions[declaration.function].return_ty),
                    declaration.rejections.join(", ")
                ),
            ))
        })
}

fn diagnostic_span(span: SourceSpan) -> Result<Span, String> {
    let start = u32::try_from(span.start_byte())
        .map_err(|_| "a current entry source span start exceeds the HIR range".to_string())?;
    let end = u32::try_from(span.end_byte())
        .map_err(|_| "a current entry source span end exceeds the HIR range".to_string())?;
    Ok(Span::new(start, end))
}

fn invalid_hir_diagnostic(message: impl Into<String>) -> Diagnostic {
    Diagnostic::without_span(
        DiagnosticSeverity::Error,
        0,
        format!("invalid HIR entry relation: {}", message.into()),
    )
}
