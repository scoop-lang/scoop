use super::*;
use crate::call_resolution::constraints::{InferenceFailure, InferenceInvariantErrorKind};
use crate::call_resolution::specificity::{ApplicableDeclaration, DeclarationForwardingView};
use crate::overload::{CallArgumentProtocol, OverloadCall, OverloadResolutionOutcome};

#[test]
fn foreign_type_parameters_are_internal_in_solving_and_satisfiability() {
    let mut lowerer = nominals::lowerer();
    let foreign = parameter(100, 0).id;
    let ty = lowerer.intern_type(Type::Param(foreign));
    let mut session = InferenceSession::new();
    session.push(
        Constraint::Equal(TypeTerm::Type(ty), lowerer.boolean.into()),
        ConstraintOrigin::Declaration,
    );
    let InferenceFailure::Internal(error) = lowerer.solve_constraints(&session).unwrap_err() else {
        panic!("a missing declaration binder is not an ordinary mismatch");
    };
    assert!(
        matches!(error.kind, InferenceInvariantErrorKind::ForeignTypeParameter(id) if id == foreign)
    );
    assert!(
        matches!(lowerer.constraints_are_satisfiable(&session).unwrap_err().kind,
        InferenceInvariantErrorKind::ForeignTypeParameter(id) if id == foreign)
    );
}

#[test]
fn specificity_preserves_internal_failures_in_both_candidate_orders() {
    let mut lowerer = nominals::lowerer();
    let foreign = lowerer.intern_type(Type::Param(parameter(101, 0).id));
    let ordinary = [lowerer.boolean];
    let broken = [foreign];
    for types in [[&ordinary, &broken], [&broken, &ordinary]] {
        let candidates = types.map(|types| ApplicableDeclaration {
            declaration: DeclarationForwardingView::parameter_groups(&[], &[], types),
            parameterized: false,
            defaults: 0,
            vararg: false,
        });
        assert!(matches!(
            lowerer
                .most_specific_declarations(&candidates)
                .unwrap_err()
                .kind,
            InferenceInvariantErrorKind::ForeignTypeParameter(_)
        ));
    }
}

#[test]
fn a_valid_overload_cannot_hide_another_candidates_internal_failure() {
    let mut lowerer = nominals::lowerer();
    let parsed = scoop_parser::parse("fun choose() {}\nfun choose() {}").unwrap();
    let functions = parsed
        .declarations
        .iter()
        .map(|declaration| {
            let scoop_ast::Decl::Function(declaration) = declaration else {
                unreachable!()
            };
            let function = lowerer.declare_function_header(declaration.header(), 0);
            lowerer.resolve_signature(function, declaration);
            function
        })
        .collect::<Vec<_>>();
    assert!(lowerer.diagnostics.is_empty(), "{:?}", lowerer.diagnostics);
    let foreign = lowerer.intern_type(Type::Param(parameter(102, 0).id));
    for (function, ty) in [(functions[0], lowerer.boolean), (functions[1], foreign)] {
        lowerer
            .signatures
            .get_mut(&function)
            .unwrap()
            .params
            .push(crate::FnParam {
                name: scoop_ast::Ident {
                    text: "value".into(),
                    span: Span::new(0, 5),
                },
                calling: crate::FnParamCalling::Required,
                ty,
            });
    }
    let arguments = [scoop_ast::CallArgument::positional(
        scoop_ast::Expr::BoolLiteral {
            value: true,
            span: Span::new(7, 11),
        },
    )];
    for candidates in [[functions[0], functions[1]], [functions[1], functions[0]]] {
        let mut state = lowerer.clone();
        let expected_result = Some(state.unit);
        let outcome = state.resolve_overload_outcome(
            "choose",
            &candidates,
            &[],
            OverloadCall {
                explicit_type_args: &[],
                arg_exprs: &arguments,
                span: Span::new(0, 8),
                expected_result,
                argument_protocol: CallArgumentProtocol::Ordinary,
            },
            &mut Vec::new(),
        );
        assert!(matches!(outcome, OverloadResolutionOutcome::Failed));
        assert_eq!(state.diagnostics.len(), 1, "{:?}", state.diagnostics);
        assert_eq!(
            state.diagnostics[0].kind(),
            scoop_ast::DiagnosticKind::Internal,
            "{:?}",
            state.diagnostics
        );
        assert!(state.diagnostics[0].message.contains("type parameter"));
        assert!(!state.diagnostics[0].message.contains("no applicable"));
    }
}
