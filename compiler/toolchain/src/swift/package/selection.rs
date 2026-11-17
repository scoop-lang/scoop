use super::*;

impl Packages {
    pub(super) fn selected_targets(
        &self,
        request: &SwiftBuildRequest,
        descriptions: &BTreeMap<PathBuf, Description>,
        plan: &BuildPlan,
    ) -> Result<BTreeMap<PathBuf, BTreeSet<String>>, ToolchainError> {
        let mut selected: BTreeMap<_, BTreeSet<_>> = descriptions
            .iter()
            .map(|(path, description)| {
                let targets = description
                    .targets
                    .iter()
                    .filter(|target| {
                        let root = path.join(&target.path);
                        plan.inputs.iter().any(|input| input.starts_with(&root))
                    })
                    .map(|target| target.name.clone())
                    .collect();
                (path.clone(), targets)
            })
            .collect();
        let product = self.manifests[&request.package]
            .products
            .iter()
            .find(|product| product["name"].as_str() == Some(&request.product))
            .expect("selected Swift product was checked before building");
        selected
            .get_mut(&request.package)
            .expect("root description")
            .extend(
                product["targets"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .map(str::to_owned),
            );
        let mut pending = selected
            .iter()
            .flat_map(|(path, targets)| {
                targets
                    .iter()
                    .map(move |target| (path.clone(), target.clone()))
            })
            .collect::<Vec<_>>();
        let mut visited = BTreeSet::new();
        while let Some((path, name)) = pending.pop() {
            if !visited.insert((path.clone(), name.clone())) {
                continue;
            }
            selected
                .get_mut(&path)
                .expect("resolved package")
                .insert(name.clone());
            let description = &descriptions[&path];
            let target = description
                .targets
                .iter()
                .find(|target| target.name == name)
                .ok_or_else(|| {
                    error(format!(
                        "Swift target {name} missing from {}",
                        path.display()
                    ))
                })?;
            for dependency in &target.target_dependencies {
                if description.targets.iter().any(|target| {
                    target.name == *dependency
                        && (matches!(target.module_type.as_str(), "PluginTarget" | "MacroTarget")
                            || target.kind == "macro")
                }) {
                    pending.push((path.clone(), dependency.clone()));
                }
            }
            let raw = self.manifests[&path]
                .targets
                .iter()
                .find(|target| target["name"].as_str() == Some(&name))
                .ok_or_else(|| error(format!("Swift target {name} has no manifest description")))?;
            for usage in raw["pluginUsages"].as_array().into_iter().flatten() {
                let plugin = usage["plugin"][0]
                    .as_str()
                    .ok_or_else(|| error("invalid Swift plugin usage"))?;
                let package = usage["plugin"][1].as_str();
                if package.is_none()
                    && description
                        .targets
                        .iter()
                        .any(|target| target.name == plugin)
                {
                    pending.push((path.clone(), plugin.to_owned()));
                    continue;
                }
                let mut candidates = Vec::new();
                for (identity, dependency) in &self.dependencies[&path] {
                    let manifest = &self.manifests[dependency];
                    if package
                        .is_some_and(|name| !self.matches_package(&path, identity, manifest, name))
                    {
                        continue;
                    }
                    if let Some(product) = manifest
                        .products
                        .iter()
                        .find(|product| product["name"].as_str() == Some(plugin))
                    {
                        candidates.push((dependency, product));
                    }
                }
                let [(owner, product)] = candidates.as_slice() else {
                    return Err(error(format!(
                        "cannot locate unique Swift plugin {plugin} in direct dependencies of {}",
                        path.display()
                    )));
                };
                for target in product["targets"].as_array().into_iter().flatten() {
                    let target = target
                        .as_str()
                        .ok_or_else(|| error("invalid Swift plugin product target"))?;
                    pending.push(((*owner).clone(), target.to_owned()));
                }
            }
        }
        Ok(selected)
    }

    pub(super) fn matches_package(
        &self,
        owner: &Path,
        identity: &str,
        manifest: &Manifest,
        name: &str,
    ) -> bool {
        if identity == name.to_ascii_lowercase() || manifest.name == name {
            return true;
        }
        self.manifests[owner].dependencies.iter().any(|dependency| {
            dependency
                .as_object()
                .into_iter()
                .flat_map(|object| object.values())
                .flat_map(|value| value.as_array().into_iter().flatten())
                .any(|entry| {
                    entry["identity"].as_str() == Some(identity)
                        && entry["nameForTargetDependencyResolution"].as_str() == Some(name)
                })
        })
    }
}
