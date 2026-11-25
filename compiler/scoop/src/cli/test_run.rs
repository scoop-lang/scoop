use std::fs::{self, File};
use std::io::{self, BufReader};
use std::path::Path;
use std::process::{Command, ExitStatus, Stdio};

use scoop::{BuildFailure, BuildFailurePhase, BuildResult};
use scoop_process::CommandExt;

use super::{presentation::Reporter, test_selection::SelectedTests};
use model::{Output, RunResult, Status};

mod events;
pub(super) mod model;

pub(super) struct Options {
    pub isolate: bool,
    pub nocapture: bool,
}

pub(super) fn program(
    executable: &Path,
    output: &Path,
    selected: &SelectedTests<'_>,
    options: Options,
    cwd: &Path,
    reporter: &Reporter,
    target: &str,
) -> BuildResult<Option<ExitStatus>> {
    if selected.case_indices.is_empty() {
        let mut result = RunResult::empty(0);
        result.complete = true;
        reporter
            .test_report(target, selected, &result, None)
            .map_err(|error| failure(scoop_source::DiagnosticCode::E2201, error))?;
        return Ok(None);
    }
    let (result, status) = run_cases(executable, output, &selected.case_indices, &options, cwd)?;
    reporter
        .test_report(target, selected, &result, Some(status))
        .map_err(|error| failure(scoop_source::DiagnosticCode::E2201, error))?;
    if status.success() && result.failed() {
        return Err(failure(
            scoop_source::DiagnosticCode::E2508,
            "test run failed; see the test report",
        ));
    }
    Ok(Some(status))
}

fn run_cases(
    executable: &Path,
    output: &Path,
    keys: &[usize],
    options: &Options,
    cwd: &Path,
) -> BuildResult<(RunResult, ExitStatus)> {
    let parent = output.parent().ok_or_else(|| {
        failure(
            scoop_source::DiagnosticCode::E2201,
            format!(
                "test executable path has no parent directory: {}",
                output.display()
            ),
        )
    })?;
    let run = tempfile::Builder::new()
        .prefix(".test-run-")
        .tempdir_in(parent)
        .map_err(|error| failure(scoop_source::DiagnosticCode::E2201, error))?;
    let keys: Vec<_> = keys.iter().map(usize::to_string).collect();
    let mut command = Command::new(executable);
    if options.isolate {
        command.arg("--scoop-test-isolate-all");
    }
    command
        .arg("--scoop-test-cases")
        .args(&keys)
        .current_dir(cwd)
        .stdin(Stdio::inherit())
        .env("SCOOP_TEST_RUN_DIR", run.path())
        .env_remove("SCOOP_TEST_NOCAPTURE")
        .env_remove("SCOOP_TEST_CASE_RESULT");
    if options.nocapture {
        command
            .env("SCOOP_TEST_NOCAPTURE", "1")
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());
    } else {
        command
            .stdout(
                File::create(run.path().join("target.stdout"))
                    .map_err(|error| failure(scoop_source::DiagnosticCode::E2201, error))?,
            )
            .stderr(
                File::create(run.path().join("target.stderr"))
                    .map_err(|error| failure(scoop_source::DiagnosticCode::E2201, error))?,
            );
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.arg0(output);
    }
    let mut child = command
        .scoop_spawn()
        .map_err(|error| failure(scoop_source::DiagnosticCode::E2202, error))?;
    let status = child
        .wait()
        .map_err(|error| failure(scoop_source::DiagnosticCode::E2204, error))?;
    drop(child);
    let expected: Vec<_> = keys.iter().map(String::as_str).collect();
    let mut result = match File::open(run.path().join("events.jsonl")) {
        Ok(file) => events::read(BufReader::new(file), &expected),
        Err(error) => {
            let mut result = RunResult::empty(expected.len());
            result.target_failure = Some(format!("cannot read test events: {error}"));
            result
        }
    };
    // Exit 1 is the generated entry's ordinary assertion/exception failure status.
    // All other nonzero exits, and exit 1 after passing cases, are target failures.
    let ordinary_failure = status.code() == Some(1)
        && result.complete
        && result
            .cases
            .iter()
            .any(|case| case.status == Status::Failed);
    if !status.success() && !ordinary_failure && result.target_failure.is_none() {
        result.target_failure = Some(format!("test process failed: {status}"));
    }
    if !options.nocapture {
        result.output = Some(
            read_output(run.path(), "target")
                .map_err(|error| failure(scoop_source::DiagnosticCode::E2201, error))?,
        );
        for (index, case) in result.cases.iter_mut().enumerate() {
            if case.status == Status::Failed {
                case.output = Some(
                    read_output(run.path(), &index.to_string())
                        .map_err(|error| failure(scoop_source::DiagnosticCode::E2201, error))?,
                );
            }
        }
    }
    Ok((result, status))
}

fn read_output(directory: &Path, prefix: &str) -> io::Result<Output> {
    fn read(path: &Path) -> io::Result<Vec<u8>> {
        match fs::read(path) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
            result => result,
        }
    }
    Ok(Output {
        stdout: read(&directory.join(format!("{prefix}.stdout")))?,
        stderr: read(&directory.join(format!("{prefix}.stderr")))?,
    })
}

fn failure(code: scoop_source::DiagnosticCode, error: impl std::fmt::Display) -> Box<BuildFailure> {
    BuildFailure::tool(code, BuildFailurePhase::Run, error)
}
