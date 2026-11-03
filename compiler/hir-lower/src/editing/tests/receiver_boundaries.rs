use super::candidates::call_candidates;
use super::receivers::{known_type, receiver};
use super::*;

#[test]
fn incomplete_calls_only_offer_accessible_receiver_members_and_extensions() {
    for declarations in [
        "class Outer { fun action(flag: Boolean) {} }",
        "class Outer\nfun Outer.action(flag: Boolean) {}",
        "class Outer(val action: (Boolean) -> Unit)",
        "class Outer\nval Outer.action: (Boolean) -> Unit get() = { flag -> }",
    ] {
        for (callee, count) in [("action", 0), ("this@editing.action", 1)] {
            let text = format!(
                "{declarations}\nclass Inner\nfun Outer.editing() {{ val block: Inner.() -> Unit = {{ {callee}( }} }}"
            );
            let candidates = call_candidates(&text);
            assert_eq!(candidates.len(), count, "{text}: {candidates:?}");
        }
    }
}

#[test]
fn receiver_queries_keep_explicit_outer_members_and_reject_implicit_ones() {
    for (value, allowed) in [("value", false), ("this@editing.value", true)] {
        let text = format!(
            "class Outer(val value: Boolean)\nclass Inner\nfun Outer.editing() {{ val block: Inner.() -> Unit = {{ {value}. }} }}"
        );
        let mut analysis = analyze(&text);
        let facts = receiver(&mut analysis);
        if allowed {
            assert_eq!(known_type(&mut analysis, &facts), "Boolean");
        } else {
            assert!(
                matches!(facts.value_type, V::Unavailable(U::BodyFailure)),
                "{facts:?}"
            );
        }
    }
}
