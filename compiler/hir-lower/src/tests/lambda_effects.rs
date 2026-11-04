use super::*;

fn check(source: &str) -> hir::ExportHirOutput {
    lower_user(scoop_parser::parse(source).unwrap()).unwrap()
}

fn body<'a>(module: &'a hir::Module, name: &str) -> &'a hir::Body {
    let function = module
        .functions
        .iter()
        .find(|(_, function)| function.name == name)
        .unwrap()
        .1;
    let hir::FunctionKind::User(body) = &function.kind else {
        panic!("expected a source body")
    };
    body
}

fn selected_suspend(module: &hir::Module, expression: &hir::Expr) -> bool {
    let hir::ExprKind::Call {
        callee: hir::CallableTarget::Local(callee),
        ..
    } = expression.kind
    else {
        panic!("expected a local call: {expression:?}")
    };
    module.functions[module.callable_function(callee)].is_suspend
}

#[test]
fn expected_effect_is_fixed_in_lambda_signature_and_definition() {
    let module = check(
        r#"
        suspend fun later(): Int = 7
        fun accept(block: suspend () -> Int) {}
        fun returned(): suspend () -> Int = { later() }
        fun main() {
            val suspended: suspend () -> Int = { later() }
            val pureSuspend: suspend () -> Int = { 1 }
            val ordinary = { 2 }
            val branch: suspend () -> Int = if (true) ({ later() }) else ({ 3 })
            accept { later() }
        }
    "#,
    );
    for (name, expected) in [
        ("suspended", true),
        ("pureSuspend", true),
        ("ordinary", false),
        ("branch", true),
    ] {
        let ty = local_init(body(&module, "main"), name).ty;
        let hir::Type::Function(id) = module.types[ty] else {
            panic!("expected a function")
        };
        assert_eq!(module.function_types[id].is_suspend, expected, "{name}");
    }
    for (_, lambda) in module.lambdas.iter() {
        assert_eq!(
            module.function_types[lambda.function_type].is_suspend,
            module.functions[lambda.definition.source_function()].is_suspend
        );
    }
}

#[test]
fn symmetric_overloads_prefer_ordinary_but_check_each_body_independently() {
    let module = check(
        r#"
        suspend fun later(): Int = 7
        fun choose(block: () -> Int): Int = block()
        suspend fun choose(block: suspend () -> Int): Int = block()
        suspend fun scenario() {
            val pure = choose { 1 }
            val suspended = choose { later() }
            val explicit = choose(suspend { 2 })
            val anonymous = choose(fun(): Int { return 3 })
        }
        fun main() { val pure = choose { 1 } }
    "#,
    );
    for (name, expected) in [
        ("pure", false),
        ("suspended", true),
        ("explicit", true),
        ("anonymous", false),
    ] {
        assert_eq!(
            selected_suspend(&module, local_init(body(&module, "scenario"), name)),
            expected,
            "{name}"
        );
    }
}

#[test]
fn generic_receiver_overloads_compare_binders_not_inferred_results() {
    check(
        r#"
        interface Carrier { fun read(): Int }
        class Resource : Carrier { public override fun read(): Int = 11 }
        fun <T: Carrier, R> T.scope(block: T.() -> R): R = this.block()
        suspend fun <U: Carrier, V> U.scope(block: suspend U.() -> V): V = this.block()
        suspend fun later(): Int = 7
        suspend fun scenario() {
            val plain = Resource().scope { read() }
            val suspended = Resource().scope { later() + read() }
        }
        fun main() {}
    "#,
    );
}

#[test]
fn lambda_effect_defaults_are_a_positionwise_partial_order() {
    check(
        r#"
        fun choose(a: () -> Int, b: () -> Int): Int = 1
        fun choose(a: suspend () -> Int, b: () -> Int): Int = 2
        fun choose(a: () -> Int, b: suspend () -> Int): Int = 3
        fun main() { choose({ 1 }, { 2 }) }
    "#,
    );
    rejects(
        r#"
        fun choose(a: suspend () -> Int, b: () -> Int): Int = 2
        fun choose(a: () -> Int, b: suspend () -> Int): Int = 3
        fun main() { choose({ 1 }, { 2 }) }
    "#,
        "ambiguous",
    );
}

#[test]
fn non_symmetric_candidates_remain_ambiguous() {
    for declarations in [
        "fun choose(block: () -> Int): Int = 1\nfun choose(block: suspend () -> Int): Long = 2L",
        "fun <T> choose(value: T, block: () -> Int): Int = 1\nfun <U> choose(value: U, block: suspend () -> U): Int = 2",
        "fun choose(value: Any, block: () -> Int): Int = 1\nfun choose(value: Int, block: suspend () -> Int): Int = 2",
    ] {
        let arguments = if declarations.contains("value:") {
            "1, { 2 }"
        } else {
            "{ 2 }"
        };
        rejects(
            &format!("{declarations}\nfun main() {{ choose({arguments}) }}"),
            "ambiguous",
        );
    }
}

#[test]
fn explicit_and_pretyped_effects_remain_boundaries() {
    for source in [
        "fun main() { val callback: suspend () -> Int = fun(): Int { return 1 } }",
        "fun main() { val plain = { 1 }; val callback: suspend () -> Int = plain }",
        "fun main() { val callback: () -> Int = suspend { 1 } }",
        "suspend fun scenario() { val callback = { later() } }\nfun main() {}",
        "suspend fun scenario() { ordinary { later() } }\nfun main() {}",
        "fun main() { suspended { later() } }",
    ] {
        rejects(
            &format!(
                r#"
            suspend fun later(): Int = 7
            fun ordinary(block: () -> Int) {{}}
            suspend fun suspended(block: suspend () -> Int) {{}}
            {source}
        "#
            ),
            "suspend",
        );
    }
}

#[test]
fn member_and_constructor_overloads_share_effect_defaults() {
    check(
        r#"
        class Builder {
            public constructor(block: () -> Int) {}
            public constructor(block: suspend () -> Int) {}
            public fun choose(block: () -> Int): Int = 1
            public fun choose(block: suspend () -> Int): Int = 2
        }
        fun main() {
            val builder = Builder { 1 }
            builder.choose { 2 }
        }
    "#,
    );
}

fn rejects(source: &str, message: &str) {
    let errors = lower_user(scoop_parser::parse(source).unwrap()).unwrap_err();
    assert!(
        errors.iter().any(|error| error.message.contains(message)),
        "{errors:#?}"
    );
}
