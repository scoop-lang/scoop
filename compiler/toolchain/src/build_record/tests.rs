use super::*;

#[test]
fn long_paths_publish_reuse_and_update_native_records() {
    let temp = tempfile::tempdir().unwrap();
    let directory = temp
        .path()
        .join("long-native-cache-directory-".repeat(3))
        .join("long-native-cache-directory-".repeat(3));
    let key = sha256(b"long-path");
    let record_path = directory.join(format!("{key}.json"));
    assert!(record_path.as_os_str().len() > 260);
    assert!(!record_path.to_string_lossy().starts_with(r"\\?\"));
    let input = temp.path().join("input.c");
    let output = temp.path().join("output.obj");
    for content in ["first native input", "changed native input"] {
        std::fs::write(&input, content).unwrap();
        let record = cached(
            &directory,
            key,
            |_: &String| Ok(output.is_file()),
            || {
                std::fs::write(&output, content).unwrap();
                Ok(Record {
                    inputs: vec![Input::read(&input)?],
                    result: content.to_owned(),
                })
            },
        )
        .unwrap();
        assert_eq!(record.result, content);
        let reused = cached(
            &directory,
            key,
            |_: &String| Ok(output.is_file()),
            || panic!("unchanged native input must reuse its result"),
        )
        .unwrap();
        assert_eq!(reused.result, content);
        assert_eq!(std::fs::read_to_string(&output).unwrap(), content);
    }
}
