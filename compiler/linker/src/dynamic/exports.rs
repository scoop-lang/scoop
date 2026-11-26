use super::*;
use crate::native_input::NativeContent;

impl DynamicInputs {
    pub fn candidates(
        &self,
        native: &NativeInputs,
        symbol: &str,
        input: Option<&[NativeInputId]>,
    ) -> Result<Vec<DynamicBinding>, LinkError> {
        let roots: Vec<_> = if let Some(inputs) = input {
            inputs
                .iter()
                .filter_map(|input| match &native.files[input].content {
                    NativeContent::Dynamic(records) => records.first().map(|provider| provider.id),
                    NativeContent::System => Some(self.system),
                    _ => None,
                })
                .collect()
        } else {
            self.roots.iter().copied().collect()
        };
        let mut result = Vec::new();
        let mut definitions = BTreeSet::new();
        for root in roots {
            if let Some(binding) = self.find(root, symbol, &mut BTreeSet::new())?
                && definitions.insert((
                    &self.providers[&binding.source].install_name,
                    binding.source_symbol.clone(),
                ))
            {
                result.push(binding);
            }
        }
        Ok(result)
    }

    fn find(
        &self,
        id: NativeDynamicProviderId,
        symbol: &str,
        visited: &mut BTreeSet<(NativeDynamicProviderId, String)>,
    ) -> Result<Option<DynamicBinding>, LinkError> {
        if !visited.insert((id, symbol.into())) {
            return Ok(None);
        }
        let provider = &self.providers[&id];
        if let Some(export) = provider.exports.get(symbol) {
            return match export {
                DynamicExport::Symbol { interface, storage } => Ok(Some(DynamicBinding {
                    owner: id,
                    source: id,
                    source_symbol: symbol.into(),
                    interface: *interface,
                    storage: *storage,
                })),
                DynamicExport::Reexport {
                    dependency,
                    imported,
                } => {
                    let child = self.edges.get(&(id, *dependency)).ok_or_else(|| {
                        error(
                            scoop_source::DiagnosticCode::E2305,
                            format!("native renamed re-export {symbol} has no resolved dependency"),
                        )
                    })?;
                    let Some(child) = child.provider() else {
                        return Ok(None);
                    };
                    self.find(child, imported, visited).map(|binding| {
                        binding.map(|mut binding| {
                            if binding.owner == child {
                                binding.owner = id;
                            }
                            binding
                        })
                    })
                }
                DynamicExport::Previous {
                    dependency,
                    interface,
                } => {
                    let child = self
                        .edges
                        .get(&(id, *dependency))
                        .and_then(|edge| edge.provider())
                        .ok_or_else(|| {
                            error(
                                scoop_source::DiagnosticCode::E2305,
                                format!("previous SDK export {symbol} has no load provider"),
                            )
                        })?;
                    Ok(Some(DynamicBinding {
                        owner: child,
                        source: child,
                        source_symbol: symbol.to_owned(),
                        interface: *interface,
                        storage: ExportStorage::InterfaceOnly,
                    }))
                }
            };
        }
        for (index, dependency) in provider.dependencies.iter().enumerate() {
            if dependency.kind == LoadKind::Reexport
                && let Some(child) = self.edges[&(id, index)].provider()
                && let Some(mut binding) = self.find(child, symbol, visited)?
            {
                if binding.owner == child {
                    binding.owner = id;
                }
                return Ok(Some(binding));
            }
        }
        Ok(None)
    }
}
