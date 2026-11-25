//! Join compiled declarations to process outcomes once, in the selected case order.

use super::*;
use crate::cli::test_run::model::CaseResult;
use scoop_slib::{TestCaseListing, TestContainerListing};

pub(super) enum Outcome<'a> {
    Ignored(&'a str),
    Executed(&'a CaseResult),
}

pub(super) struct Container<'a> {
    pub declaration: &'a TestContainerListing,
    pub cases: Vec<(&'a TestCaseListing, Outcome<'a>)>,
    pub status: &'static str,
}

#[derive(Default)]
pub(super) struct Counts {
    pub passed: usize,
    pub failed: usize,
    pub not_run: usize,
    pub ignored: usize,
}

impl Counts {
    pub fn cases(&self) -> usize {
        self.passed + self.failed + self.not_run + self.ignored
    }
}

pub(super) struct Report<'a> {
    pub containers: Vec<Container<'a>>,
    pub counts: Counts,
    pub empty_reason: Option<&'static str>,
}

impl<'a> Report<'a> {
    pub fn new(selected: &SelectedTests<'a>, result: &'a RunResult) -> io::Result<Self> {
        let mut outcomes = result.cases.iter();
        let mut counts = Counts::default();
        let mut containers = Vec::with_capacity(selected.containers.len());
        for group in &selected.containers {
            let mut cases = Vec::with_capacity(group.cases.len());
            let mut status = None;
            for &case in &group.cases {
                let outcome = if let Some(reason) = &case.ignore_reason
                    && !selected.run_ignored
                {
                    counts.ignored += 1;
                    Outcome::Ignored(reason)
                } else {
                    let result = outcomes.next().ok_or_else(|| {
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("missing test outcome for {}", case.display_name),
                        )
                    })?;
                    match result.status {
                        Status::Passed => counts.passed += 1,
                        Status::NotRun => counts.not_run += 1,
                        Status::Failed => counts.failed += 1,
                    }
                    status = status.max(Some(result.status));
                    Outcome::Executed(result)
                };
                cases.push((case, outcome));
            }
            containers.push(Container {
                declaration: group.declaration,
                cases,
                status: status.map_or("ignored", Status::label),
            });
        }
        if outcomes.next().is_some() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "test run returned more outcomes than selected cases",
            ));
        }
        Ok(Self {
            containers,
            counts,
            empty_reason: selected.empty_reason,
        })
    }
}
