use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::inputs::{Rule, Selection};
use super::plan::BuildPlan;
use super::{
    Input, Produced, Record, SwiftBuildRequest, SwiftLibraryFormat, SwiftLinkage, ToolchainError,
    build_record, error, outputs_exist, values,
};
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
    mut dependencies: Vec<crate::NativeLibraryDependency>,
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
    dependencies.sort_by(|left, right| left.library.cmp(&right.library));
    dependencies.dedup_by(|left, right| left.library == right.library);
    libraries.extend(
        dependencies
            .iter()
            .map(|dependency| dependency.library.clone()),
    );
    let mut names = BTreeSet::new();
    for library in &libraries {
        if !names.insert(library.file_name()) {
            return Err(error(format!(
                "conflicting Swift native output {}",
                library.display()
            )));
        }
    }
    let binary = snapshot(request, "binaries", &libraries)?;
    for (dependency, published) in dependencies.iter_mut().zip(&binary.files[1..]) {
        let relative = dependency
            .binary
            .strip_prefix(&dependency.library)
            .map_err(error)?;
        dependency.binary = if relative.as_os_str().is_empty() {
            published.clone()
        } else {
            published.join(relative)
        };
        dependency.library = published.clone();
    }
    let module = bin.join(format!("{}.swiftmodule", request.product));
    let modules = snapshot(
        request,
        "modules",
        &if module.is_dir() {
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
    let link_key = binary.key;
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
                dependencies,
                resources: resources.files,
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
        error("framework requires a built Swift module with the same name as the product")
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
