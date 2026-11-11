use super::*;

#[test]
fn api_projection_keeps_signatures_bindings_constraints_and_missing_documentation() {
    let mut nominal = item("nominal", "assertEquals", ApiKind::Function, None);
    nominal.signature[0].text =
        "public fun <T: Equals<T>> assertEquals(expected: T, actual: T)".into();
    let mut structural = item("structural", "assertEquals", ApiKind::Function, None);
    structural.signature[0].text = "public fun <T> assertEquals(expected: T, actual: T) where requires(x: T) { x.equals(x): Boolean }".into();
    structural.comment = Some(comment(
        "Compare actual and expected. Use `scoop example test/assertions`.",
    ));
    structural.receiver = Some(SignatureToken {
        text: "Option<T>".into(),
        link: None,
    });
    let mut hidden = item("hidden", "hidden", ApiKind::Function, None);
    hidden.hidden = true;
    let mut property = item("state", "state", ApiKind::Property, None);
    property.property = Some(PropertyCapability {
        getter: AccessorCapability {
            key: SymbolKey("getter".into()),
            visibility: Visibility::Public,
            hidden: false,
        },
        setter: Some(AccessorCapability {
            key: SymbolKey("setter".into()),
            visibility: Visibility::Private,
            hidden: false,
        }),
    });
    let mut input = cone("a", vec![nominal, structural, hidden, property]);
    input.bindings = vec![
        ExportBinding {
            package: vec!["public_api".into()],
            name: "assert".into(),
            target: SymbolKey("structural".into()),
            reexport: true,
        },
        ExportBinding {
            package: vec!["public_api".into()],
            name: "hidden".into(),
            target: SymbolKey("hidden".into()),
            reexport: true,
        },
    ];
    let inputs = [input];
    let catalog = SiteCatalog::new(&inputs, SiteOptions::default()).unwrap();
    let renderer = SiteRenderer::new(&catalog).unwrap();
    let index: scoop_api::Index =
        serde_json::from_slice(&renderer.api_index("aarch64-apple-darwin").unwrap().contents)
            .unwrap();
    assert_eq!(index.target, "aarch64-apple-darwin");
    let data: scoop_api::Cone =
        serde_json::from_slice(&renderer.api_data(&inputs[0]).unwrap().contents).unwrap();
    assert_eq!(data.declarations.len(), 3);
    assert_eq!(data.bindings.len(), 1);
    assert!(data.bindings[0].reexport);
    let property = data
        .declarations
        .iter()
        .find(|d| d.key == "state")
        .unwrap()
        .property
        .as_ref()
        .unwrap();
    assert_eq!(property.getter.visibility, "public");
    assert_eq!(property.setter.as_ref().unwrap().visibility, "private");
    assert_eq!(
        data.declarations[0].source.path.as_deref(),
        Some("src/用户.scoop")
    );
    assert!(data.declarations.iter().any(|d| d.documentation.is_none()));
    let structural = data
        .declarations
        .iter()
        .find(|d| d.key == "structural")
        .unwrap();
    assert!(structural.signature.contains("where requires"));
    assert_eq!(structural.receiver.as_deref(), Some("Option<T>"));
    assert!(
        structural
            .documentation
            .as_ref()
            .unwrap()
            .markdown
            .contains("test/assertions")
    );
}
