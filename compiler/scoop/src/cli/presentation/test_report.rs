use std::process::ExitStatus;

use super::*;
use crate::cli::test_run::model::{Output, ProcessStatus, RunResult, Status};
use crate::cli::test_selection::SelectedTests;

mod human;
mod summary;
use summary::{Outcome, Report};
#[cfg(test)]
mod tests;

impl Reporter {
    pub fn test_report(
        &self,
        target: &str,
        selected: &SelectedTests,
        result: &RunResult,
        process: Option<ExitStatus>,
    ) -> io::Result<()> {
        let report = Report::new(selected, result)?;
        let process = process.map(ProcessStatus::from);
        match self.0 {
            MessageFormat::Human => {
                human::write(&mut io::stdout().lock(), target, &report, result, process)
            }
            MessageFormat::Json => write_json(&json_report(target, &report, result, process)),
        }
    }
}

fn json_report(
    target: &str,
    report: &Report<'_>,
    result: &RunResult,
    process: Option<ProcessStatus>,
) -> Value {
    let containers = report.containers.iter().map(|group| {
        let cases = group.cases.iter().map(|(case, outcome)| {
            let mut record = json!({
                "name": case.name, "display_name": case.display_name,
                "location": test_list::location(&case.location), "ignore_reason": case.ignore_reason,
            });
            match outcome {
                Outcome::Ignored(_) => record["status"] = json!("ignored"),
                Outcome::Executed(result) => {
                    record["status"] = json!(result.status);
                    record["process"] = json!(result.process);
                    record["duration_ns"] = json!(result.duration_ns);
                    record["failures"] = json!(result.failures);
                    record["interruption"] = json!(result.interruption);
                    record["output"] = output(result.output.as_ref());
                }
            }
            record
        }).collect::<Vec<_>>();
        json!({"name": group.declaration.qualified_name, "status": group.status,
            "location": test_list::location(&group.declaration.location), "cases": cases})
    }).collect::<Vec<_>>();
    let counts = &report.counts;
    json!({
        "schema": 2, "kind": "test-report", "target": target,
        "status": if result.failed() { "failed" } else { "passed" },
        "complete": result.complete, "empty_reason": report.empty_reason, "containers": containers,
        "counts": {"containers": containers.len(), "cases": counts.cases(), "passed": counts.passed,
            "failed": counts.failed, "not_run": counts.not_run, "ignored": counts.ignored},
        "target_failure": result.target_failure, "output": output(result.output.as_ref()), "process": process,
    })
}

impl From<ExitStatus> for ProcessStatus {
    fn from(status: ExitStatus) -> Self {
        Self {
            code: status.code(),
            #[cfg(unix)]
            signal: std::os::unix::process::ExitStatusExt::signal(&status),
            #[cfg(not(unix))]
            signal: None,
        }
    }
}

fn output(value: Option<&Output>) -> Value {
    value.map_or(Value::Null, |value| {
        json!({
            "stdout": bytes(&value.stdout), "stderr": bytes(&value.stderr),
        })
    })
}

fn bytes(value: &[u8]) -> Value {
    match std::str::from_utf8(value) {
        Ok(text) => json!({"text": text}),
        Err(_) => {
            json!({"hex": value.iter().map(|byte| format!("{byte:02x}")).collect::<String>()})
        }
    }
}
