use super::*;

pub(crate) fn complete<'a>(
    consumer: ConeIdentity,
    target: crate::LirTargetProfile,
    dependencies: &[&'a LayoutAbiExportConstituentsV1],
) -> Result<Vec<&'a LayoutAbiExportConstituentsV1>, LayoutAbiSectionError> {
    let mut tables = reserve(dependencies.len())?;
    tables.extend_from_slice(dependencies);
    tables.sort_unstable_by_key(|table| table.provider());
    let mut previous = None;
    for table in &tables {
        let provider = table.provider();
        if provider == consumer || previous == Some(provider) {
            return Err(LayoutAbiSectionError::DuplicateProvider(provider));
        }
        if table.target_profile() != target {
            return Err(LayoutAbiSectionError::DependencyTarget { provider });
        }
        previous = Some(provider);
    }
    Ok(tables)
}
