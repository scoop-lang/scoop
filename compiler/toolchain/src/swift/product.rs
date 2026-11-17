use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::inputs::{Rule, Selection};
use super::plan::BuildPlan;
use super::tool::SwiftTools;
use super::{
    Input, Produced, Record, SwiftBuildRequest, SwiftLibraryFormat, SwiftLinkage, ToolchainError,
    build_record, error, outputs_exist, values,
};
use crate::NativeLinkSettings;
use scoop_wire::Digest256;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SwiftProduct {
    #[serde(flatten)]
    pub output: crate::NativeLibraryOutput,
    #[serde(with = "values::digest")]
    pub producer_key: Digest256,
    #[serde(with = "values::digest")]
    pub link_key: Digest256,
    #[serde(with = "values::digest")]
    pub deployment_key: Digest256,
}

impl SwiftProduct {
    pub fn outputs(products: &[Self]) -> Result<Vec<crate::NativeLibraryOutput>, ToolchainError> {
        let mut selected = std::collections::BTreeMap::new();
        for product in products {
            if let Some(previous) = selected.insert(&product.output.name, product)
                && previous.producer_key != product.producer_key
            {
                return Err(ToolchainError::new(
                    scoop_source::DiagnosticCode::E2306,
                    format!(
                        "conflicting Swift producers for native library {}: {} and {}",
                        product.output.name,
                        previous.output.library.display(),
                        product.output.library.display()
                    ),
                ));
            }
        }
        Ok(selected
            .into_values()
            .map(|product| product.output.clone())
            .collect())
    }
}

#[derive(Serialize, Deserialize)]
struct Snapshot {
    #[serde(with = "values::digest")]
    key: Digest256,
    #[serde(with = "values::paths")]
    files: Vec<PathBuf>,
    #[serde(with = "values::paths")]
    required: Vec<PathBuf>,
}

pub(super) fn publish(
    request: &SwiftBuildRequest,
    bin: &Path,
    plan: &BuildPlan,
    selections: Vec<Selection>,
    settings: NativeLinkSettings,
    single_module: bool,
) -> Result<Produced, ToolchainError> {
    let bin = bin.canonicalize().map_err(error)?;
    let main = bin.join(format!(
        "lib{}.{}",
        request.product,
        match request.linkage {
            SwiftLinkage::Static => "a",
            SwiftLinkage::Dynamic => "dylib",
        }
    ));
    let mut libraries = vec![main.clone()];
    libraries.extend(
        plan.outputs
            .iter()
            .filter(|path| {
                path.parent() == Some(&bin)
                    && path.extension().is_some_and(|ext| ext == "dylib")
                    && **path != main
            })
            .cloned(),
    );
    let binary = snapshot(request, "binaries", &libraries)?;
    let module = bin.join(format!("{}.swiftmodule", request.product));
    let modules = snapshot(
        request,
        "modules",
        &if single_module && module.is_dir() {
            vec![module]
        } else {
            Vec::new()
        },
    )?;
    let resource_roots = plan
        .outputs
        .iter()
        .filter_map(|path| {
            let relative = path.strip_prefix(&bin).ok()?;
            let first = relative.components().next()?.as_os_str();
            Path::new(first)
                .extension()
                .is_some_and(|ext| ext == "bundle")
                .then(|| bin.join(first))
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let resources = snapshot(request, "resources", &resource_roots)?;
    let link_key = build_record::fingerprint(&(binary.key.as_array(), &settings))?;
    let mut required = binary.required;
    required.extend(modules.required);
    required.extend(resources.required);
    Ok(Produced {
        product: SwiftProduct {
            producer_key: request.key,
            link_key,
            deployment_key: resources.key,
            output: crate::NativeLibraryOutput {
                name: request.product.clone(),
                linkage: request.linkage,
                format: SwiftLibraryFormat::Library,
                library: binary.files[0].clone(),
                binary: binary.files[0].clone(),
                dependencies: binary.files[1..]
                    .iter()
                    .cloned()
                    .map(crate::NativeLibraryDependency::dylib)
                    .collect::<Result<_, _>>()?,
                resources: resources.files,
                settings,
            },
        },
        selections,
        required,
        modules: modules.files,
    })
}

fn snapshot(
    request: &SwiftBuildRequest,
    role: &str,
    files: &[PathBuf],
) -> Result<Snapshot, ToolchainError> {
    let mut inputs = Vec::new();
    for file in files {
        let selection = Selection::read(file.clone(), Rule::All)?;
        inputs.push(Input::read(file)?);
        inputs.extend(
            selection
                .files()
                .iter()
                .map(|file| Input::read(file))
                .collect::<Result<Vec<_>, _>>()?,
        );
    }
    let key = build_record::fingerprint(&(role, &inputs))?;
    Ok(build_record::cached(
        &request.directory.join("snapshots"),
        key,
        |result: &Snapshot| outputs_exist(&result.required),
        || {
            let directory = tempfile::Builder::new()
                .prefix(role)
                .tempdir_in(&request.directory)
                .map_err(error)?;
            let mut outputs = Vec::new();
            let mut required = Vec::new();
            for input in files {
                let output = directory.path().join(
                    input
                        .file_name()
                        .ok_or_else(|| error("Swift output has no file name"))?,
                );
                copy(input, &output, &mut required)?;
                outputs.push(output);
            }
            let _ = directory.keep();
            Ok(Record {
                inputs: Vec::new(),
                result: Snapshot {
                    key,
                    files: outputs,
                    required,
                },
            })
        },
    )?
    .result)
}

fn layout_key(request: &SwiftBuildRequest, result: &Produced) -> Result<Digest256, ToolchainError> {
    build_record::fingerprint(&(
        request.key.as_array(),
        &request.format,
        result.product.link_key.as_array(),
        result
            .modules
            .iter()
            .map(|p| scoop_protocol::HostPathCarrier::from_path(p))
            .collect::<Result<Vec<_>, _>>()
            .map_err(error)?,
    ))
}

pub(super) fn lookup_layout(
    request: &SwiftBuildRequest,
    result: &Produced,
) -> Result<Option<SwiftProduct>, ToolchainError> {
    if matches!(request.format, SwiftLibraryFormat::Library) {
        return Ok(Some(result.product.clone()));
    }
    let key = layout_key(request, result)?;
    let record = build_record::lookup::<Layout>(
        &request
            .directory
            .join("layouts")
            .join(format!("{key}.json")),
        |layout| outputs_exist(&layout.required),
    )?;
    Ok(record.map(|record| with_resources(record.result.product, result)))
}

#[derive(Serialize, Deserialize)]
struct Layout {
    product: SwiftProduct,
    #[serde(with = "values::paths")]
    required: Vec<PathBuf>,
}

pub(super) fn layout(
    request: &SwiftBuildRequest,
    result: &Produced,
) -> Result<SwiftProduct, ToolchainError> {
    let SwiftLibraryFormat::Framework { bundle_id } = &request.format else {
        return Ok(result.product.clone());
    };
    let module = result.modules.first().ok_or_else(|| {
        error("framework requires one direct Swift module with the same name as the product")
    })?;
    let key = layout_key(request, result)?;
    Ok(with_resources(build_record::cached(&request.directory.join("layouts"), key, |layout: &Layout| outputs_exist(&layout.required), || {
        let directory = tempfile::Builder::new().prefix("framework").tempdir_in(&request.directory).map_err(error)?;
        let framework = directory.path().join(format!("{}.framework", request.product));
        let version = framework.join("Versions/A");
        let binary = version.join(&request.product);
        let mut required = Vec::new();
        copy(&result.product.output.binary, &binary, &mut required)?;
        copy(module, &version.join("Modules").join(format!("{}.swiftmodule", request.product)), &mut required)?;
        let plist = version.join("Resources/Info.plist");
        std::fs::create_dir_all(plist.parent().expect("framework Resources")).map_err(error)?;
        let xml = format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?><!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\"><plist version=\"1.0\"><dict><key>CFBundleIdentifier</key><string>{bundle_id}</string><key>CFBundleName</key><string>{0}</string><key>CFBundleExecutable</key><string>{0}</string><key>CFBundlePackageType</key><string>FMWK</string></dict></plist>\n", xml_escape(&request.product));
        std::fs::write(&plist, xml).map_err(error)?;
        required.push(plist);
        symlink(Path::new("A"), &framework.join("Versions/Current"))?;
        for entry in [&request.product, "Resources", "Modules"] {
            let link = framework.join(entry);
            symlink(&Path::new("Versions/Current").join(entry), &link)?;
            required.push(link);
        }
        if request.linkage == SwiftLinkage::Dynamic {
            super::tool::run(std::process::Command::new("/usr/bin/install_name_tool").arg("-id").arg(format!("@rpath/{}.framework/Versions/A/{}", request.product, request.product)).arg(&binary))?;
            super::tool::run(std::process::Command::new("/usr/bin/codesign").args(["--force", "--sign", "-"]).arg(&framework))?;
            required.extend(Selection::read(framework.clone(), Rule::All)?.files().iter().cloned());
        }
        let mut product = result.product.clone();
        product.output.format = request.format.clone(); product.output.library = framework; product.output.binary = binary;
        product.producer_key = build_record::fingerprint(&(request.key.as_array(), &request.format))?;
        product.link_key = build_record::fingerprint(&(result.product.link_key.as_array(), &request.format))?;
        let _ = directory.keep();
        Ok(Record { inputs: Vec::new(), result: Layout { product, required } })
    })?.result.product, result))
}

fn with_resources(mut product: SwiftProduct, result: &Produced) -> SwiftProduct {
    product.output.resources = result.product.output.resources.clone();
    product.deployment_key = result.product.deployment_key;
    product
}

pub(super) fn copy(
    source: &Path,
    destination: &Path,
    required: &mut Vec<PathBuf>,
) -> Result<(), ToolchainError> {
    let metadata = source.symlink_metadata().map_err(error)?;
    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent).map_err(error)?;
    }
    if metadata.file_type().is_symlink() {
        symlink(&std::fs::read_link(source).map_err(error)?, destination)?;
    } else if metadata.is_dir() {
        std::fs::create_dir_all(destination).map_err(error)?;
        for child in std::fs::read_dir(source).map_err(error)? {
            let child = child.map_err(error)?;
            copy(
                &child.path(),
                &destination.join(child.file_name()),
                required,
            )?;
        }
    } else {
        std::fs::copy(source, destination).map_err(error)?;
    }
    required.push(destination.to_path_buf());
    Ok(())
}

fn symlink(source: &Path, destination: &Path) -> Result<(), ToolchainError> {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(source, destination).map_err(error)
    }
    #[cfg(not(unix))]
    {
        let _ = (source, destination);
        Err(error("Swift frameworks require a macOS host"))
    }
}

fn xml_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

impl NativeLinkSettings {
    pub(super) fn add_manifest_target(
        &mut self,
        target: &Value,
        configuration: &str,
    ) -> Result<(), ToolchainError> {
        for setting in target["settings"].as_array().into_iter().flatten() {
            if setting["tool"].as_str() != Some("linker") {
                continue;
            }
            let condition = &setting["condition"];
            if condition["config"]
                .as_str()
                .is_some_and(|c| c != configuration)
                || condition["platformNames"]
                    .as_array()
                    .is_some_and(|ps| !ps.iter().any(|p| p.as_str() == Some("macos")))
            {
                continue;
            }
            let kind = &setting["kind"];
            if let Some(library) = kind["linkedLibrary"]["_0"].as_str() {
                self.libraries.insert(library.to_owned());
            } else if let Some(framework) = kind["linkedFramework"]["_0"].as_str() {
                self.frameworks.insert(framework.to_owned());
            } else if !kind.is_null() {
                return Err(error(format!(
                    "Swift target {:?} has unsupported consumer linker setting {kind}",
                    target["name"]
                )));
            }
        }
        Ok(())
    }

    pub(super) fn add_plan(
        &mut self,
        plan: &BuildPlan,
        scratch: &Path,
        tools: &SwiftTools,
    ) -> Result<(), ToolchainError> {
        for command in &plan.commands {
            if !command.description.starts_with("Ld ") {
                continue;
            }
            for option in &command.args {
                let target = if let Some(path) = option.strip_prefix("-L") {
                    Some((&mut self.library_paths, path))
                } else {
                    option
                        .strip_prefix("-F")
                        .map(|path| (&mut self.framework_paths, path))
                };
                if let Some((roots, path)) = target {
                    if path.is_empty() {
                        continue;
                    }
                    let path = PathBuf::from(path);
                    let path = path.canonicalize().unwrap_or(path);
                    if !path.starts_with(scratch)
                        && !tools.is_tool_input(&path)
                        && !roots.contains(&path)
                    {
                        roots.push(path);
                    }
                }
            }
        }
        Ok(())
    }
}
