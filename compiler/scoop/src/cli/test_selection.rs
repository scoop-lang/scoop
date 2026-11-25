use scoop_slib::{TestCaseListing, TestContainerListing, TestDirectory};

#[cfg(test)]
mod tests;

#[derive(Clone, Copy, PartialEq)]
pub(super) enum Purpose {
    List,
    Run,
}

pub(super) struct SelectedContainer<'a> {
    pub declaration: &'a TestContainerListing,
    pub cases: Vec<&'a TestCaseListing>,
}

pub(super) struct SelectedTests<'a> {
    pub containers: Vec<SelectedContainer<'a>>,
    /// Indices in the compiled executable, including gaps left by the filter.
    pub case_indices: Vec<usize>,
    pub run_ignored: bool,
    pub empty_reason: Option<&'static str>,
}

pub(super) fn select<'a>(
    directory: &'a TestDirectory,
    filter: Option<&str>,
    ignored: bool,
    purpose: Purpose,
) -> SelectedTests<'a> {
    let mut containers = Vec::new();
    let mut case_indices = Vec::new();
    let mut next_index = 0;
    let mut any_match = false;
    for declaration in &directory.containers {
        let mut cases = Vec::new();
        for case in &declaration.cases {
            let index = next_index;
            next_index += 1;
            if filter.is_some_and(|name| !case.display_name.contains(name)) {
                continue;
            }
            any_match = true;
            if case.ignore_reason.is_some() == ignored {
                case_indices.push(index);
                cases.push(case);
            } else if purpose == Purpose::Run && !ignored {
                // Default run reports also explain which matching cases were skipped.
                cases.push(case);
            }
        }
        if !cases.is_empty() {
            containers.push(SelectedContainer { declaration, cases });
        }
    }
    let empty_reason = if !case_indices.is_empty() {
        None
    } else if next_index == 0 {
        Some("no tests")
    } else if !any_match {
        Some("no matching tests")
    } else if ignored {
        Some("no ignored tests")
    } else {
        Some("all matching tests ignored")
    };
    SelectedTests {
        containers,
        case_indices,
        run_ignored: ignored,
        empty_reason,
    }
}
