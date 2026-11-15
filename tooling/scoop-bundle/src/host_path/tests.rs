use super::*;

#[test]
fn current_member_paths_and_build_output_paths_remain_lossless() {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;

        let expected = PathBuf::from(OsString::from_vec(vec![b'/', b'a', 0xff]));
        let carrier = json!({"encoding": "UnixBytes", "bytes": "L2H/"});
        assert_eq!(decode(&carrier).unwrap(), expected);
        assert_eq!(value(&expected), carrier);
        assert_eq!(
            decode(&json!({"encoding": "unix-bytes", "hex": "2f61ff"})).unwrap(),
            expected
        );
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStringExt;

        let expected = PathBuf::from(OsString::from_wide(&[67, 58, 92, 0xd800]));
        let carrier = json!({"encoding": "WindowsWtf16Le", "bytes": "QwA6AFwAANg="});
        assert_eq!(decode(&carrier).unwrap(), expected);
        assert_eq!(value(&expected), carrier);
        assert_eq!(
            decode(&json!({"encoding": "windows-wide", "units": [67, 58, 92, 0xd800]})).unwrap(),
            expected
        );
    }
    let unicode = PathBuf::from("editor 中文 with spaces");
    assert_eq!(decode(&value(&unicode)).unwrap(), unicode);
}

#[test]
fn malformed_member_path_bytes_are_rejected() {
    let encoding = if cfg!(windows) {
        "WindowsWtf16Le"
    } else {
        "UnixBytes"
    };
    for bytes in [json!("%%%"), json!(""), json!("AAA="), json!([97])] {
        assert!(decode(&json!({"encoding": encoding, "bytes": bytes})).is_err());
    }
    #[cfg(windows)]
    assert!(decode(&json!({"encoding": encoding, "bytes": "YQ=="})).is_err());
}
