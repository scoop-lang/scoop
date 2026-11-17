//! SwiftPM owns package resolution. These projections describe actual inputs.
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::Value;

use super::inputs::{Rule, Selection, add_selection};
use super::plan::BuildPlan;
use super::tool::SwiftTools;
use super::{Input, SwiftBuildRequest, SwiftLibraryFormat, SwiftLinkage, ToolchainError, error};
use crate::NativeLinkSettings;

#[derive(Deserialize)]
struct Manifest {
    #[serde(default)]
    dependencies: Vec<Value>,
    #[serde(default)]
    platforms: Vec<Value>,
    #[serde(default)]
    products: Vec<Value>,
    #[serde(default)]
    targets: Vec<Value>,
}

#[derive(Deserialize)]
struct Description {
    targets: Vec<Target>,
}

#[derive(Deserialize)]
struct Target {
    name: String,
    path: PathBuf,
    #[serde(default)]
    sources: Vec<PathBuf>,
    #[serde(default)]
    target_dependencies: Vec<String>,
    #[serde(default)]
    module_type: String,
}

#[derive(Deserialize)]
struct Dependency {
    path: PathBuf,
    #[serde(default)]
    dependencies: Vec<Dependency>,
}

pub(super) struct Packages {
    manifests: BTreeMap<PathBuf, Manifest>,
}

impl Packages {
    pub(super) fn has_framework_module(&self, request: &SwiftBuildRequest) -> bool {
        self.manifests[&request.package]
            .products
            .iter()
            .find(|product| product["name"].as_str() == Some(&request.product))
            .is_some_and(|product| {
                product["targets"].as_array().is_some_and(|targets| {
                    targets.len() == 1 && targets[0].as_str() == Some(&request.product)
                })
            })
    }

    pub(super) fn prepare(
        request: &SwiftBuildRequest,
        tools: &SwiftTools,
        scratch: &Path,
    ) -> Result<Self, ToolchainError> {
        let root: Manifest =
            tools.package_json(&request.package, scratch, &["dump-package"], false)?;
        let product = root
            .products
            .iter()
            .find(|p| p["name"].as_str() == Some(&request.product))
            .ok_or_else(|| {
                error(format!(
                    "product {:?} is not declared in {}",
                    request.product,
                    request.package.display()
                ))
            })?;
        let expected = match request.linkage {
            SwiftLinkage::Static => "static",
            SwiftLinkage::Dynamic => "dynamic",
        };
        if product["type"]["library"][0].as_str() != Some(expected) {
            return Err(error(format!(
                "product {:?} must explicitly declare .library(type: .{expected}); automatic products do not provide an independent library",
                request.product
            )));
        }
        if matches!(request.format, SwiftLibraryFormat::Framework { .. })
            && (product["targets"]
                .as_array()
                .is_none_or(|targets| targets.len() != 1)
                || product["targets"][0].as_str() != Some(&request.product))
        {
            return Err(error(
                "framework requires one direct Swift target with the same name as the product",
            ));
        }
        check_deployment(&root, tools, &request.package)?;
        let mut result = Self {
            manifests: BTreeMap::from([(request.package.clone(), root)]),
        };
        if request.options.offline {
            result.check_local(&request.package, tools, scratch)?;
        }
        Ok(result)
    }

    fn check_local(
        &mut self,
        path: &Path,
        tools: &SwiftTools,
        scratch: &Path,
    ) -> Result<(), ToolchainError> {
        let manifest = &self.manifests[path];
        if manifest
            .targets
            .iter()
            .any(|target| target["url"].as_str().is_some())
        {
            return Err(error(
                "offline Swift build cannot acquire a remote binary target; build online first",
            ));
        }
        let mut children = Vec::new();
        for dependency in &manifest.dependencies {
            let Some(path) = dependency["fileSystem"][0]["path"].as_str() else {
                return Err(error(
                    "this SwiftPM has no local-only dependency-fetch mode; an uncached offline build requires only local path dependencies",
                ));
            };
            children.push(PathBuf::from(path));
        }
        for child in children {
            let child = child.canonicalize().map_err(error)?;
            if !self.manifests.contains_key(&child) {
                let manifest = tools.package_json(&child, scratch, &["dump-package"], false)?;
                self.manifests.insert(child.clone(), manifest);
                self.check_local(&child, tools, scratch)?;
            }
        }
        Ok(())
    }

    pub(super) fn discover(
        &mut self,
        request: &SwiftBuildRequest,
        tools: &SwiftTools,
        scratch: &Path,
    ) -> Result<(), ToolchainError> {
        let graph: Dependency = tools.package_json(
            &request.package,
            scratch,
            &["show-dependencies", "--format", "json"],
            request.options.locked || request.shared_source,
        )?;
        let mut pending = vec![graph];
        while let Some(dependency) = pending.pop() {
            let path = dependency.path.canonicalize().map_err(error)?;
            if let std::collections::btree_map::Entry::Vacant(entry) = self.manifests.entry(path) {
                let manifest =
                    tools.package_json(entry.key(), scratch, &["dump-package"], false)?;
                entry.insert(manifest);
            }
            pending.extend(dependency.dependencies);
        }
        Ok(())
    }

    pub(super) fn inputs(
        &self,
        request: &SwiftBuildRequest,
        tools: &SwiftTools,
        scratch: &Path,
        plan: &BuildPlan,
    ) -> Result<(Vec<Input>, Vec<Selection>, NativeLinkSettings), ToolchainError> {
        let mut files = BTreeSet::from([
            request.package.join("Package.resolved"),
            scratch.join("workspace-state.json"),
        ]);
        let mut selections = Vec::new();
        let mut settings = NativeLinkSettings::default();
        for (path, manifest) in &self.manifests {
            add_selection(path.clone(), Rule::Manifests, &mut files, &mut selections)?;
            add_selection(
                path.join(".swiftpm/configuration"),
                Rule::All,
                &mut files,
                &mut selections,
            )?;
            let description: Description =
                tools.package_json(path, scratch, &["describe", "--type", "json"], false)?;
            let targets = description
                .targets
                .iter()
                .map(|t| (t.name.as_str(), t))
                .collect::<BTreeMap<_, _>>();
            let mut selected = description
                .targets
                .iter()
                .filter(|target| {
                    let root = path.join(&target.path);
                    target
                        .sources
                        .iter()
                        .any(|source| plan.inputs.contains(&root.join(source)))
                        || plan.inputs.iter().any(|source| source.starts_with(&root))
                })
                .map(|t| t.name.clone())
                .collect::<BTreeSet<_>>();
            // Plugins and macros may run outside the destination build plan.
            loop {
                let mut added = Vec::new();
                for name in &selected {
                    if let Some(target) = targets.get(name.as_str()) {
                        added.extend(
                            target
                                .target_dependencies
                                .iter()
                                .filter(|name| {
                                    targets.get(name.as_str()).is_some_and(|t| {
                                        matches!(
                                            t.module_type.as_str(),
                                            "PluginTarget" | "MacroTarget"
                                        )
                                    })
                                })
                                .cloned(),
                        );
                    }
                    if let Some(raw) = manifest
                        .targets
                        .iter()
                        .find(|t| t["name"].as_str() == Some(name))
                        && let Some(plugins) = raw["pluginUsages"].as_array()
                    {
                        for plugin in plugins {
                            if let Some(name) = plugin["plugin"][0].as_str()
                                && targets.contains_key(name)
                            {
                                added.push(name.to_owned());
                            }
                        }
                    }
                }
                let old = selected.len();
                selected.extend(added);
                if selected.len() == old {
                    break;
                }
            }
            if !selected.is_empty() {
                check_deployment(manifest, tools, path)?;
            }
            for name in selected {
                let target = targets[name.as_str()];
                if path == &request.package
                    && name == request.product
                    && matches!(request.format, SwiftLibraryFormat::Framework { .. })
                    && target.module_type != "SwiftTarget"
                {
                    return Err(error("framework product must expose a Swift module"));
                }
                let raw = manifest
                    .targets
                    .iter()
                    .find(|t| t["name"].as_str() == Some(&name))
                    .ok_or_else(|| {
                        error(format!(
                            "Swift target {name} missing from package description"
                        ))
                    })?;
                let root = path.join(&target.path);
                let paths = |key: &str| {
                    raw[key]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(Value::as_str)
                        .map(PathBuf::from)
                        .collect::<Vec<_>>()
                };
                let mut excluded = paths("exclude");
                if let Some(resources) = raw["resources"].as_array() {
                    for resource in resources {
                        if let Some(relative) = resource["path"].as_str() {
                            excluded.push(PathBuf::from(relative));
                            add_selection(
                                root.join(relative),
                                Rule::All,
                                &mut files,
                                &mut selections,
                            )?;
                        }
                    }
                }
                add_selection(
                    root,
                    Rule::Sources {
                        include: paths("sources"),
                        exclude: excluded,
                    },
                    &mut files,
                    &mut selections,
                )?;
                if request.linkage == SwiftLinkage::Static {
                    settings.add_manifest_target(raw, request.configuration)?;
                }
            }
        }
        // The normal SwiftPM configuration location can affect resolution even
        // when the package has no package-local configuration yet.
        if let Some(home) = std::env::var_os("HOME") {
            add_selection(
                PathBuf::from(home).join(".swiftpm/configuration"),
                Rule::All,
                &mut files,
                &mut selections,
            )?;
        }
        for input in &request.inputs {
            if !input.try_exists().map_err(error)? {
                return Err(error(format!(
                    "declared input does not exist: {}",
                    input.display()
                )));
            }
            add_selection(input.clone(), Rule::All, &mut files, &mut selections)?;
        }
        let scratch = scratch.canonicalize().map_err(error)?;
        for input in plan.inputs.iter().chain(&plan.dependencies) {
            if tools.is_tool_input(input) {
                continue;
            }
            let generated = input.starts_with(&scratch)
                && !self.manifests.keys().any(|root| input.starts_with(root));
            if !generated && input.is_file() {
                files.insert(input.clone());
            }
        }
        settings.add_plan(plan, &scratch, tools)?;
        let inputs = files
            .iter()
            .map(|path| Input::read(path))
            .collect::<Result<_, _>>()?;
        Ok((inputs, selections, settings))
    }
}

fn check_deployment(
    manifest: &Manifest,
    tools: &SwiftTools,
    package: &Path,
) -> Result<(), ToolchainError> {
    for platform in &manifest.platforms {
        if platform["platformName"].as_str() == Some("macos") {
            let version = platform["version"]
                .as_str()
                .ok_or_else(|| error("missing package deployment version"))?;
            let version =
                crate::c_bridge::parse_darwin_version(version, "package deployment version")?;
            if version > tools.minimum_os {
                return Err(error(format!(
                    "{} requires macOS {version}, above selected deployment {}",
                    package.display(),
                    tools.minimum_os
                )));
            }
        }
    }
    Ok(())
}
