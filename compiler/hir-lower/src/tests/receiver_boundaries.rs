use super::*;

const DIAGNOSTIC: &str = "implicit use of an outer receiver is not allowed";

fn source(body: &str) -> String {
    format!(
        r#"
        class Inner
        struct Action()
        operator fun Action.invoke(value: Int): Int = value
        class Outer(var number: Int) {{
            val handler: (Int) -> Int = {{ it }}
            val action: Action = Action()
            fun method(): Int = number
            inner class Child
            fun run(): Int {{
                {body}
                return 0
            }}
        }}
        var Outer.extension: Int
            get() = number
            set(value) {{ number = value }}
        fun Outer.extensionCall(): Int = number
        val Outer.extensionHandler: (Int) -> Int get() = {{ number + it }}
        val Outer.extensionAction: Action get() = Action()
        fun main() {{}}
        "#
    )
}

fn assert_rejected(text: &str, token: &str) {
    let parsed = scoop_parser::parse(text).unwrap();
    let errors = lower_user(parsed).expect_err("implicit outer receiver must be rejected");
    let error = errors
        .iter()
        .find(|error| error.message.contains(DIAGNOSTIC))
        .unwrap_or_else(|| panic!("missing outer receiver diagnostic: {errors:#?}"));
    let span = error.span.expect("receiver diagnostic has a source span");
    assert_eq!(&text[span.start as usize..span.end as usize], token);
}

#[test]
fn receiver_boundaries_reject_implicit_member_and_extension_uses_at_the_name() {
    for (body, token) in [
        ("number", "number"),
        ("number = 3; 0", "number"),
        ("number += 3; 0", "number"),
        ("number++; 0", "number"),
        ("method()", "method"),
        ("handler(3)", "handler"),
        ("action(3)", "action"),
        ("extension", "extension"),
        ("extension = 3; 0", "extension"),
        ("extension += 3; 0", "extension"),
        ("extensionCall()", "extensionCall"),
        ("extensionHandler(3)", "extensionHandler"),
        ("extensionAction(3)", "extensionAction"),
    ] {
        assert_rejected(
            &source(&format!("val block: Inner.() -> Int = {{ {body} }}")),
            token,
        );
    }
}

#[test]
fn receiver_boundaries_apply_to_anonymous_suspend_and_nested_plain_callables() {
    for body in [
        "val block = fun Inner.(): Int { return number }",
        "val block: suspend Inner.() -> Int = suspend { number }",
        "val block = suspend fun Inner.(): Int { return number }",
        "val block: Inner.() -> Int = { val callback = { number }; callback() }",
        "val block: Inner.() -> Int = { fun callback(): Int = number; callback() }",
    ] {
        assert_rejected(&source(body), "number");
    }
}

#[test]
fn receiver_boundaries_reject_generic_outer_members() {
    assert_rejected(
        r#"
        class Inner
        class Outer<T>(val value: T)
        fun <T> Outer<T>.capture(): Inner.() -> T = { value }
        fun main() {}
        "#,
        "value",
    );
}

#[test]
fn receiver_boundaries_reject_implicit_enclosing_instances() {
    for body in [
        "val block: Inner.() -> Unit = { Child() }",
        "val block: Inner.() -> Unit = { val create = ::Child }",
    ] {
        let text = source(body);
        let errors = lower_user(scoop_parser::parse(&text).unwrap()).unwrap_err();
        assert!(
            errors
                .iter()
                .any(|error| error.message.contains(DIAGNOSTIC)),
            "{errors:#?}"
        );
    }
}

#[test]
fn receiver_boundaries_preserve_explicit_receivers_and_lexical_captures() {
    let text = source(
        r#"
        val owner = this
        val earlier = { number }
        fun earlierFunction(): Int = number
        val ordinary: (Inner) -> Int = { ignored -> number }
        val adapted: Inner.() -> Int = ordinary
        val block: Inner.() -> Int = {
            owner.number += 1
            this@Outer.extension += 1
            val method = owner::method
            val create = owner::Child
            create()
            val callback = { this@Outer.number }
            callback() + earlier() + earlierFunction() + adapted(this) + method()
        }
        val same: Outer.() -> Int = outer@ {
            val nested: Outer.() -> Int = {
                number + this@outer.number + owner.number
            }
            nested(Outer(2))
        }
        val result = block(Inner()) + same(Outer(3))
        "#,
    );
    lower_user(scoop_parser::parse(&text).unwrap())
        .expect("explicit and lexical captures remain legal");
}

#[test]
fn receiver_boundaries_do_not_reject_unselected_outer_candidates() {
    let text = r#"
        class Outer {
            fun choose(value: Int): Int = value
            fun run(): Boolean {
                val block: Inner.() -> Boolean = { choose(true) }
                return block(Inner())
            }
        }
        class Inner
        fun choose(value: Boolean): Boolean = value
        fun Outer.extension(value: Int): Int = value
        fun extension(value: Boolean): Boolean = value
        fun Outer.test(): Boolean {
            val block: Inner.() -> Boolean = { extension(true) }
            return block(Inner())
        }
        fun main() {}
    "#;
    lower_user(scoop_parser::parse(text).unwrap())
        .expect("inapplicable outer candidates do not fail lookup");
}

#[test]
fn receiver_boundaries_do_not_fall_back_from_a_selected_outer_member() {
    let text = source("val block: Inner.() -> Int = { method() }") + "\nfun method(): Int = 99\n";
    assert_rejected(&text, "method");
}

#[test]
fn receiver_boundaries_do_not_replace_an_inner_constructor_with_a_function() {
    let text = source("val block: Inner.() -> Unit = { Child() }") + "\nfun Child(): Int = 99\n";
    let errors = lower_user(scoop_parser::parse(&text).unwrap())
        .expect_err("an inner constructor cannot silently bind a different function");
    assert!(
        errors
            .iter()
            .any(|error| error.message.contains(DIAGNOSTIC)),
        "{errors:#?}"
    );
}

#[test]
fn receiver_boundaries_allow_current_function_properties_before_outer_methods() {
    let text = r#"
        class Inner(val choose: () -> Int)
        class Outer {
            fun choose(): Int = 1
            fun run(): Int {
                val block: Inner.() -> Int = { choose() }
                return block(Inner { 42 })
            }
        }
        fun main() {}
    "#;
    lower_user(scoop_parser::parse(text).unwrap())
        .expect("current function property wins normally");
}
