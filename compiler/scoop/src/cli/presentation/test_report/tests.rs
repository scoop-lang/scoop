use super::*;
use crate::cli::test_run::model::{CaseResult, Failure, FailureDetail, Location, Phase};
use crate::cli::test_selection::{Purpose, select};
use scoop_slib::{TestCaseListing, TestContainerListing, TestDirectory, TestSourceLocation};

#[test]
fn output_encoding_preserves_control_bytes_and_invalid_utf8() {
    assert_eq!(bytes(b"\0\n\""), json!({"text": "\0\n\""}));
    assert_eq!(bytes(&[0, 255, 10]), json!({"hex": "00ff0a"}));
    assert_eq!(output(None), Value::Null);
}

#[test]
fn incomplete_human_reports_show_the_actual_exit_or_signal() {
    let directory = TestDirectory::default();
    let selected = select(&directory, None, false, Purpose::Run);
    let mut result = RunResult::empty(0);
    result.target_failure = Some("missing completion".into());
    let report = Report::new(&selected, &result).unwrap();
    for (process, expected) in [
        (
            ProcessStatus {
                code: Some(0),
                signal: None,
            },
            "process exit: 0",
        ),
        (
            ProcessStatus {
                code: None,
                signal: Some(6),
            },
            "process signal: 6",
        ),
    ] {
        let mut text = Vec::new();
        human::write(&mut text, "demo", &report, &result, Some(process)).unwrap();
        assert!(String::from_utf8(text).unwrap().contains(expected));
    }
}

#[test]
fn human_report_keeps_failures_locations_ignored_reasons_and_raw_output() {
    let location = TestSourceLocation {
        path: "tests/cases.scoop".into(),
        line: 3,
        column: 1,
    };
    let case = |name: &str, ignore_reason: Option<String>| TestCaseListing {
        name: name.into(),
        display_name: format!("demo::Cases::{name}"),
        location: location.clone(),
        ignore_reason,
    };
    let directory = TestDirectory {
        containers: vec![TestContainerListing {
            qualified_name: "Cases".into(),
            location: location.clone(),
            cases: vec![case("check", None), case("later", Some("waiting".into()))],
        }],
    };
    let selected = select(&directory, None, false, Purpose::Run);
    let result = RunResult {
        complete: true,
        target_failure: None,
        output: None,
        cases: vec![CaseResult {
            status: Status::Failed,
            duration_ns: Some(17),
            interruption: None,
            failures: vec![Failure {
                phase: Phase::AfterTest,
                location: Location {
                    file: "tests/cleanup.scoop".into(),
                    line: 9,
                    column: 4,
                    function: "cleanup".into(),
                    r#type: "Cases".into(),
                },
                detail: FailureDetail::Exception {
                    message: "cleanup failed".into(),
                },
            }],
            output: Some(Output {
                stdout: vec![255],
                stderr: Vec::new(),
            }),
        }],
    };
    let mut text = Vec::new();
    let report = Report::new(&selected, &result).unwrap();
    human::write(&mut text, "demo", &report, &result, None).unwrap();
    let rendered = String::from_utf8_lossy(&text);
    for detail in [
        "Cases [failed]",
        "check (tests/cases.scoop:3:1) failed (17 ns)",
        "after-test at tests/cleanup.scoop:9:4",
        "cleanup failed",
        "ignored: waiting",
        "0 passed, 1 failed, 1 ignored, 0 not run",
    ] {
        assert!(
            rendered.contains(detail),
            "missing {detail:?} in {rendered}"
        );
    }
    let raw = b"case stdout:\n\xff\n";
    assert!(text.windows(raw.len()).any(|bytes| bytes == raw));
}
