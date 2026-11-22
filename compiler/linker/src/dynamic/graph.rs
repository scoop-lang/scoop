use super::*;
use crate::native_input::{NativeContent, NativeFileKind};

impl DynamicInputs {
    pub fn read(
        native: &mut NativeInputs,
        roots: &[PathBuf],
        profile: &ValidatedFinalLinkProfile,
    ) -> Result<Self, LinkError> {
        let system = profile
            .system_provider()
            .map_err(|cause| error(cause.code(), cause))?;
        let file = system
            .files()
            .iter()
            .find(|file| file.relative_path() == system.root_stub())
            .ok_or_else(|| {
                error(
                    scoop_source::DiagnosticCode::E2305,
                    "system provider has no root stub",
                )
            })?;
        let input = NativeInputId::from_digest(
            file.digest(),
            file.byte_len(),
            NativeFileKind::TextStub,
            profile,
        )?;
        let id = DynamicProvider::id(input, system.install_name(), profile)?;
        let (exports, dependencies) = super::stub::entries(
            system.exports().clone(),
            system.previous_exports(),
            Vec::new(),
        );
        let provider = Arc::new(DynamicProvider {
            id,
            input,
            install_name: system.install_name().into(),
            current_version: system.current_version(),
            compatibility_version: system.compatibility_version(),
            exports,
            dependencies,
            rpaths: Vec::new(),
            locator: profile
                .startup_toolchain()
                .sdk_root()
                .map_err(|cause| error(scoop_source::DiagnosticCode::ICE2201, cause))?
                .join(system.root_stub()),
        });
        let mut result = Self {
            providers: BTreeMap::from([(id, provider)]),
            roots: BTreeSet::from([id]),
            edges: BTreeMap::new(),
            rpaths: BTreeSet::new(),
            stubs: BTreeMap::new(),
        };
        result.extend(native, roots, profile)?;
        Ok(result)
    }

    pub fn extend(
        &mut self,
        native: &mut NativeInputs,
        roots: &[PathBuf],
        profile: &ValidatedFinalLinkProfile,
    ) -> Result<(), LinkError> {
        let mut names = self
            .providers
            .values()
            .map(|p| (p.install_name.clone(), p.id))
            .collect();
        for file in native.ordered_files() {
            if let NativeContent::Dynamic(records) = &file.content {
                let root = records.first().ok_or_else(|| {
                    error(
                        scoop_source::DiagnosticCode::E2305,
                        "dynamic library has no root record",
                    )
                })?;
                locate::runpath(root, roots, profile)?;
                self.roots.insert(root.id);
                for record in records {
                    self.insert(record.clone(), &mut names)?;
                }
            }
        }
        let mut pending: BTreeSet<_> = self.providers.keys().copied().collect();
        let mut visited = BTreeSet::new();
        while let Some(id) = pending.pop_first() {
            if !visited.insert(id) {
                continue;
            }
            let provider = self.providers[&id].clone();
            for (index, edge) in provider.dependencies.iter().enumerate() {
                match self.edges.get(&(id, index)) {
                    Some(LoadTarget::Provider(_)) => continue,
                    Some(LoadTarget::AbsentWeak) if !names.contains_key(&edge.name) => continue,
                    _ => {}
                }
                let target = if let Some(target) = names.get(&edge.name) {
                    *target
                } else {
                    let Some(file) = locate::dependency(
                        &provider,
                        edge,
                        roots,
                        profile,
                        &mut native.observed_paths,
                    )?
                    else {
                        self.edges.insert((id, index), LoadTarget::AbsentWeak);
                        continue;
                    };
                    let NativeContent::Dynamic(records) = &file.content else {
                        return Err(error(
                            scoop_source::DiagnosticCode::E2305,
                            "dynamic dependency resolved to a static input",
                        ));
                    };
                    let target = records
                        .iter()
                        .find(|record| record.install_name == edge.name)
                        .or_else(|| edge.name.starts_with("@loader_path/").then(|| &records[0]))
                        .ok_or_else(|| {
                            error(
                                scoop_source::DiagnosticCode::E2305,
                                format!("dynamic file does not contain {}", edge.name),
                            )
                        })?
                        .id;
                    for record in records {
                        self.insert(record.clone(), &mut names)?;
                        pending.insert(record.id);
                    }
                    native.files.entry(file.id).or_insert(file);
                    target
                };
                let child = &self.providers[&target];
                if child.current_version < edge.compatibility_version {
                    return Err(error(
                        scoop_source::DiagnosticCode::E2305,
                        format!(
                            "dynamic dependency {} has incompatible version for {}",
                            child.install_name, provider.install_name
                        ),
                    ));
                }
                self.edges.insert((id, index), LoadTarget::Provider(target));
            }
        }
        Ok(())
    }

    fn insert(
        &mut self,
        record: Arc<DynamicProvider>,
        names: &mut BTreeMap<String, NativeDynamicProviderId>,
    ) -> Result<(), LinkError> {
        if let Some(previous) = names.insert(record.install_name.clone(), record.id)
            && previous != record.id
        {
            return Err(error(
                scoop_source::DiagnosticCode::E2306,
                format!(
                    "conflicting native providers for install name {}: {} and {}",
                    record.install_name,
                    self.providers[&previous].locator.display(),
                    record.locator.display()
                ),
            ));
        }
        self.providers.entry(record.id).or_insert(record);
        Ok(())
    }

    pub fn project(
        &mut self,
        bindings: &BTreeMap<String, DynamicBinding>,
        roots: &[PathBuf],
        profile: &ValidatedFinalLinkProfile,
    ) -> Result<(), LinkError> {
        let mut imports: BTreeMap<_, BTreeMap<_, _>> = BTreeMap::new();
        for (symbol, binding) in bindings {
            imports
                .entry(binding.owner)
                .or_default()
                .insert(symbol.clone(), binding.interface);
        }
        // The fixed provider remains an explicit load input even without calls.
        let system_name = profile
            .system_provider()
            .map_err(|cause| error(cause.code(), cause))?
            .install_name();
        let system = self
            .providers
            .values()
            .find(|provider| provider.install_name == system_name)
            .ok_or_else(|| {
                error(
                    scoop_source::DiagnosticCode::E2305,
                    "missing fixed system provider",
                )
            })?;
        imports.entry(system.id).or_default();
        for id in imports.keys() {
            if let Some(path) = locate::runpath(&self.providers[id], roots, profile)? {
                self.rpaths.insert(path);
            }
        }
        let mut pending: BTreeSet<_> = imports.keys().copied().collect();
        let mut visited = BTreeSet::new();
        while let Some(id) = pending.pop_first() {
            if !visited.insert(id) {
                continue;
            }
            let provider = &self.providers[&id];
            for (index, edge) in provider.dependencies.iter().enumerate() {
                let Some(child) = self.edges[&(id, index)].provider() else {
                    continue;
                };
                if edge.name.starts_with("@rpath/")
                    && let Some(path) = locate::runpath(&self.providers[&child], roots, profile)?
                {
                    self.rpaths.insert(path);
                }
                pending.insert(child);
            }
        }
        for (id, symbols) in imports {
            let provider = &self.providers[&id];
            let stub = scoop_toolchain::write_link_stub(
                profile.target().architecture(),
                platform(profile)?,
                &provider.install_name,
                provider.current_version,
                provider.compatibility_version,
                &symbols,
            )
            .map_err(|cause| error(cause.code(), cause))?;
            self.stubs.insert(id, stub);
        }
        Ok(())
    }
}
