use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Phase {
    Initialization,
    BeforeTest,
    Body,
    AfterTest,
}

impl Phase {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Initialization => "initialization",
            Self::BeforeTest => "before-test",
            Self::Body => "body",
            Self::AfterTest => "after-test",
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Location {
    pub file: String,
    pub line: i64,
    pub column: i64,
    pub function: String,
    pub r#type: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum AssertionValue {
    Unavailable,
    Text { text: String },
    RenderingFailed { message: String },
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum FailureDetail {
    Assertion {
        assertion: String,
        message: Option<String>,
        expected: AssertionValue,
        actual: AssertionValue,
        cause: Option<String>,
    },
    Exception {
        message: String,
    },
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Failure {
    pub phase: Phase,
    pub location: Location,
    #[serde(flatten)]
    pub detail: FailureDetail,
}

/// In report order, a failure outranks an interrupted case, which outranks a pass.
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    Passed,
    #[default]
    NotRun,
    Failed,
}

impl Status {
    pub fn label(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::NotRun => "not-run",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug, Default)]
pub struct Output {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

#[derive(Debug, Default)]
pub struct CaseResult {
    pub status: Status,
    pub duration_ns: Option<u64>,
    pub process: Option<ProcessStatus>,
    pub failures: Vec<Failure>,
    pub interruption: Option<String>,
    pub output: Option<Output>,
}

#[derive(Debug)]
pub struct RunResult {
    pub cases: Vec<CaseResult>,
    pub complete: bool,
    pub target_failure: Option<String>,
    pub output: Option<Output>,
}

impl RunResult {
    pub fn empty(count: usize) -> Self {
        Self {
            cases: (0..count).map(|_| CaseResult::default()).collect(),
            complete: false,
            target_failure: None,
            output: None,
        }
    }

    pub fn failed(&self) -> bool {
        self.target_failure.is_some() || self.cases.iter().any(|case| case.status != Status::Passed)
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ProcessStatus {
    pub code: Option<i32>,
    pub signal: Option<i32>,
}
