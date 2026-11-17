use super::*;
use scoop_wire::sha256;

impl DynamicInputs {
    pub fn encode(
        &self,
        bindings: &BTreeMap<String, DynamicBinding>,
        e: &mut Encoder,
    ) -> Result<(), scoop_wire::cbor::EncodeError> {
        e.array(4)?;
        e.array(self.providers.len() as u64)?;
        for provider in self.providers.values() {
            e.array(8)?;
            provider.id.encode(e)?;
            provider.input.encode(e)?;
            e.text(&provider.install_name)?;
            e.unsigned(u64::from(provider.current_version))?;
            e.unsigned(u64::from(provider.compatibility_version))?;
            e.array(provider.dependencies.len() as u64)?;
            for (index, dependency) in provider.dependencies.iter().enumerate() {
                e.array(4)?;
                e.text(&dependency.name)?;
                e.unsigned(match dependency.kind {
                    LoadKind::Normal => 0,
                    LoadKind::Weak => 1,
                    LoadKind::Reexport => 2,
                })?;
                e.unsigned(u64::from(dependency.compatibility_version))?;
                match self.edges[&(provider.id, index)] {
                    LoadTarget::Provider(id) => {
                        e.array(1)?;
                        id.encode(e)?;
                    }
                    LoadTarget::AbsentWeak => e.array(0)?,
                }
            }
            e.array(provider.rpaths.len() as u64)?;
            for path in &provider.rpaths {
                e.text(path)?;
            }
            e.unsigned(u64::from(self.roots.contains(&provider.id)))?;
        }
        e.array(bindings.len() as u64)?;
        for (symbol, binding) in bindings {
            e.array(6)?;
            e.text(symbol)?;
            binding.owner.encode(e)?;
            binding.source.encode(e)?;
            e.text(&binding.source_symbol)?;
            e.unsigned(if binding.interface.kind == SystemExportKind::ThreadLocal {
                2
            } else {
                1
            })?;
            e.unsigned(u64::from(binding.interface.weak))?;
        }
        e.array(self.stubs.len() as u64)?;
        for (id, bytes) in &self.stubs {
            e.array(2)?;
            id.encode(e)?;
            sha256(bytes).encode(e)?;
        }
        e.array(self.rpaths.len() as u64)?;
        for path in &self.rpaths {
            e.text(path)?;
        }
        Ok(())
    }

    pub fn dump(&self, bindings: &BTreeMap<String, DynamicBinding>) -> String {
        let mut text = String::new();
        let mut providers = self.providers.values().collect::<Vec<_>>();
        providers.sort_unstable_by(|left, right| {
            left.install_name
                .cmp(&right.install_name)
                .then_with(|| left.id.cmp(&right.id))
        });
        for provider in providers {
            // Keep the established basic plan compact; native bindings below
            // still record their actual SDK owner in the canonical encoding.
            if provider.install_name == scoop_toolchain::LIBSYSTEM_INSTALL_NAME {
                continue;
            }
            text.push_str(&format!(
                "dynamic provider {} install={:?} imports={}\n",
                provider.id,
                provider.install_name,
                bindings
                    .values()
                    .filter(|binding| binding.owner == provider.id)
                    .count()
            ));
            for (index, edge) in provider.dependencies.iter().enumerate() {
                text.push_str(&format!(
                    "  {} {:?} -> {}\n",
                    match edge.kind {
                        LoadKind::Normal => "load",
                        LoadKind::Weak => "weak-load",
                        LoadKind::Reexport => "re-export",
                    },
                    edge.name,
                    match self.edges[&(provider.id, index)] {
                        LoadTarget::Provider(id) => id.to_string(),
                        LoadTarget::AbsentWeak => "absent".to_owned(),
                    }
                ));
            }
        }
        for (symbol, binding) in bindings {
            if self.providers[&binding.owner].install_name
                != scoop_toolchain::LIBSYSTEM_INSTALL_NAME
            {
                text.push_str(&format!(
                    "dynamic binding {symbol} -> {:?} source={:?}:{} kind={:?} weak={}\n",
                    self.providers[&binding.owner].install_name,
                    self.providers[&binding.source].install_name,
                    binding.source_symbol,
                    binding.interface.kind,
                    binding.interface.weak
                ));
            }
        }
        for path in &self.rpaths {
            text.push_str(&format!("rpath {path:?}\n"));
        }
        text
    }
}
