use super::*;
use crate::cli::test_selection::SelectedTests;
use scoop_slib::{TestCaseListing, TestContainerListing, TestDirectory, TestSourceLocation};

impl Reporter {
    pub fn test_list(&self, target: &str, selected: &SelectedTests<'_>) -> io::Result<()> {
        let cases = selected.case_indices.len();
        let ignored = if selected.run_ignored { cases } else { 0 };
        let groups = &selected.containers;
        match self.0 {
            MessageFormat::Json => write_json(&json!({
                "schema": 2, "kind": "test-list", "target": target,
                "containers": groups.iter().map(|group| container_record(
                    group.declaration, group.cases.iter().copied()
                )).collect::<Vec<_>>(),
                "counts": {"containers": groups.len(), "cases": cases, "ignored": ignored},
                "empty_reason": selected.empty_reason,
            })),
            MessageFormat::Human => {
                let mut out = io::stdout().lock();
                writeln!(out, "{target}")?;
                for group in groups {
                    let container = group.declaration;
                    let loc = &container.location;
                    writeln!(
                        out,
                        "  {} ({}:{}:{})",
                        container.qualified_name, loc.path, loc.line, loc.column
                    )?;
                    for case in &group.cases {
                        let loc = &case.location;
                        write!(
                            out,
                            "    {} ({}:{}:{})",
                            case.name, loc.path, loc.line, loc.column
                        )?;
                        if let Some(reason) = &case.ignore_reason {
                            write!(out, " [ignored: {reason}]")?;
                        }
                        writeln!(out)?;
                    }
                }
                write!(
                    out,
                    "{} containers, {cases} tests, {ignored} ignored",
                    groups.len()
                )?;
                if let Some(reason) = selected.empty_reason {
                    write!(out, " ({reason})")?;
                }
                writeln!(out)
            }
        }
    }
}

pub(super) fn containers(directory: &TestDirectory) -> Vec<Value> {
    directory
        .containers
        .iter()
        .map(|group| container_record(group, group.cases.iter()))
        .collect()
}

fn container_record<'a>(
    group: &TestContainerListing,
    cases: impl Iterator<Item = &'a TestCaseListing>,
) -> Value {
    json!({
        "name": group.qualified_name, "location": location(&group.location),
        "cases": cases.map(|case| json!({
            "name": case.name, "display_name": case.display_name,
            "location": location(&case.location), "ignore_reason": case.ignore_reason,
        })).collect::<Vec<_>>(),
    })
}

pub(super) fn location(location: &TestSourceLocation) -> Value {
    json!({"path": location.path, "line": location.line, "column": location.column})
}
