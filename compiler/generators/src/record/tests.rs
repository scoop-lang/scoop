use super::*;

fn simple_arguments(count: usize) -> Vec<(String, Argument)> {
    (0..count)
        .map(|index| {
            let (value_type, default) = match index % 3 {
                0 => ("Boolean", SimpleDefault::Boolean(false)),
                1 => ("Array<String>", SimpleDefault::EmptyArray),
                _ => ("Option<String>", SimpleDefault::None),
            };
            (
                format!("field{index}"),
                Argument::Defaulted {
                    presence: format!("input{index}"),
                    value_type: value_type.into(),
                    default,
                },
            )
        })
        .collect()
}

fn source(args: &[(String, Argument)]) -> String {
    let mut body = Body::default();
    body.line("input.end()");
    construct(&mut body, "Model", true, args, "Option");
    body.finish()
}

#[test]
fn simple_defaults_generate_one_constructor_with_linear_growth() {
    let small = source(&simple_arguments(10));
    let large = source(&simple_arguments(20));
    assert_eq!(small.matches("return Model(").count(), 1);
    assert_eq!(large.matches("return Model(").count(), 1);
    assert!(large.len() < small.len() * 3);
    assert!(small.starts_with("{\ninput.end()\nval "));
    assert_eq!(small.matches(" -> []").count(), 3);
    assert_eq!(small.matches(" -> Option.None").count(), 3);
}

#[test]
fn only_custom_defaults_multiply_constructor_calls() {
    let mut args = simple_arguments(10);
    args.insert(0, ("first".into(), Argument::Optional("firstInput".into())));
    args.push(("second".into(), Argument::Optional("secondInput".into())));
    args.push(("ignored".into(), Argument::Omitted));
    args.push((
        "required".into(),
        Argument::Supplied("requiredValue".into()),
    ));
    let source = source(&args);
    let calls = source
        .lines()
        .filter(|line| line.starts_with("return Model("))
        .collect::<Vec<_>>();
    assert_eq!(calls.len(), 4);
    assert_eq!(
        calls
            .iter()
            .filter(|call| call.contains("`first` ="))
            .count(),
        2
    );
    assert_eq!(
        calls
            .iter()
            .filter(|call| call.contains("`second` ="))
            .count(),
        2
    );
    for call in calls {
        assert!(call.contains("`required` = requiredValue"));
        assert!(!call.contains("`ignored`"));
        for index in 0..10 {
            assert!(call.contains(&format!("`field{index}` =")));
        }
    }
}
