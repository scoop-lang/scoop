//! Cache inputs declared by the Cone, without interpreting Package.swift.
use super::inputs::{Rule, Selection, add_selection};
use super::*;
use std::collections::BTreeSet;

pub(super) fn read(
    request: &SwiftBuildRequest,
) -> Result<(Vec<Input>, Vec<Selection>), ToolchainError> {
    let mut files = BTreeSet::from([
        request.package.join("Package.swift"),
        request.package.join("Package.resolved"),
    ]);
    let mut selections = Vec::new();
    let rule = Rule::Package {
        cache: request.directory.clone(),
    };
    add_selection(
        request.package.clone(),
        rule.clone(),
        &mut files,
        &mut selections,
    )?;
    for path in &request.inputs {
        require(path)?;
        add_selection(path.clone(), rule.clone(), &mut files, &mut selections)?;
    }
    if let Some(home) = std::env::var_os("HOME") {
        add_selection(
            PathBuf::from(home).join(".swiftpm/configuration"),
            Rule::All,
            &mut files,
            &mut selections,
        )?;
    }
    let inputs = files
        .iter()
        .map(|path| Input::read(path))
        .collect::<Result<_, _>>()?;
    Ok((inputs, selections))
}

pub(super) fn supplemental(request: &SwiftBuildRequest) -> Result<Vec<Input>, ToolchainError> {
    let mut files = BTreeSet::new();
    for path in &request.inputs {
        require(path)?;
        let selection = Selection::read(
            path.clone(),
            Rule::Package {
                cache: request.directory.clone(),
            },
        )?;
        files.extend(selection.files().iter().cloned());
    }
    files.iter().map(|path| Input::read(path)).collect()
}

fn require(path: &Path) -> Result<(), ToolchainError> {
    if !path.try_exists().map_err(error)? {
        return Err(error(format!(
            "declared input does not exist: {}",
            path.display()
        )));
    }
    Ok(())
}
