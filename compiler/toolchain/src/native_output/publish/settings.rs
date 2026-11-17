use super::*;
use crate::NativeLibraryDependency;
use scoop_manifest::SwiftLinkage;
use std::io::Read;

pub(super) fn collect(
    output: &mut NativeLibraryOutput,
    observed: &mut BTreeSet<PathBuf>,
) -> Result<(), ToolchainError> {
    let settings = &output.settings;
    let mut selected = BTreeMap::new();
    for (name, framework) in settings
        .libraries
        .iter()
        .map(|n| (n, false))
        .chain(settings.frameworks.iter().map(|n| (n, true)))
    {
        let roots = if framework {
            &settings.framework_paths
        } else {
            &settings.library_paths
        };
        let mut found = BTreeMap::new();
        for root in roots {
            let candidates = if framework {
                vec![(root.join(format!("{name}.framework")), true)]
            } else {
                vec![
                    (root.join(format!("lib{name}.a")), false),
                    (root.join(format!("lib{name}.dylib")), false),
                ]
            };
            for (library, framework) in candidates {
                let binary = if framework {
                    library.join(name)
                } else {
                    library.clone()
                };
                observed.insert(binary.clone());
                if binary.try_exists().map_err(error)? {
                    let canonical = binary.canonicalize().map_err(error)?;
                    let mut magic = [0u8; 8];
                    std::fs::File::open(&binary)
                        .map_err(error)?
                        .read_exact(&mut magic)
                        .map_err(error)?;
                    found.insert(
                        canonical,
                        NativeLibraryDependency {
                            name: name.clone(),
                            framework,
                            library,
                            binary,
                            linkage: if &magic == b"!<arch>\n" {
                                SwiftLinkage::Static
                            } else {
                                SwiftLinkage::Dynamic
                            },
                        },
                    );
                }
            }
        }
        if found.len() > 1 {
            return Err(error(format!(
                "ambiguous native dependency {name} in Swift link search paths"
            )));
        }
        if let Some(dependency) = found.into_values().next() {
            selected.insert(name.clone(), dependency);
        }
    }
    for dependency in &output.dependencies {
        if let Some(previous) = selected.insert(dependency.name.clone(), dependency.clone())
            && previous.binary != dependency.binary
        {
            return Err(error(format!(
                "conflicting native dependency {}",
                dependency.name
            )));
        }
    }
    output.dependencies = selected.into_values().collect();
    Ok(())
}
