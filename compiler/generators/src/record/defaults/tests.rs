use super::*;

fn body(operation: Value) -> Value {
    json!({"statements": [], "value": {"operation": operation}})
}

#[test]
fn direct_defaults_keep_their_values() {
    for value in [false, true] {
        assert_eq!(
            classify(
                &body(json!({"kind": "booleanLiteral", "value": value})),
                false
            ),
            Some(SimpleDefault::Boolean(value))
        );
    }
    for operation in [
        json!({"kind": "noneLiteral"}),
        json!({"kind": "variantConstruct", "arguments": []}),
    ] {
        assert_eq!(classify(&body(operation), true), Some(SimpleDefault::None));
    }
    assert_eq!(
        classify(
            &body(json!({"kind": "arrayLiteral", "elements": []})),
            false
        ),
        Some(SimpleDefault::EmptyArray)
    );
}

#[test]
fn other_defaults_keep_constructor_evaluation() {
    for operation in [
        json!({"kind": "call"}),
        json!({"kind": "local"}),
        json!({"kind": "globalRead"}),
        json!({"kind": "integerLiteral", "value": {"value": "123"}}),
        json!({"kind": "variantConstruct", "arguments": [{"value": 1}]}),
        json!({"kind": "arrayLiteral", "elements": [{"value": 1}]}),
    ] {
        assert_eq!(classify(&body(operation), true), None);
    }
    assert_eq!(
        classify(
            &body(json!({"kind": "variantConstruct", "arguments": []})),
            false
        ),
        None
    );
}

#[test]
fn a_literal_result_does_not_hide_default_side_effects() {
    let mut value = body(json!({"kind": "booleanLiteral", "value": false}));
    value["statements"] = json!([{"operation": {"kind": "expr"}}]);
    assert_eq!(classify(&value, false), None);
    assert_eq!(classify(&Value::Null, false), None);
}
