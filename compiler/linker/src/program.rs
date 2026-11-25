//! The resolved object and symbol inputs of one executable.
use std::collections::{BTreeMap, BTreeSet};

use scoop_identity::{ConeIdentity, PersistentSymbolRequest};
use scoop_slib::{LinkDefinitionOwnerV1, ProgramLinkClosure, SlibMemberId};
use scoop_toolchain::ValidatedFinalLinkProfile;

use crate::native_input::{NativeInputs, NativeObjectId};
use crate::{LinkError, RuntimeObjectId, RuntimeObjectSet, error};

pub(crate) mod image;
pub(crate) mod native;
mod selection;
use selection::SelectedMembers;
mod object;
pub(crate) use object::{InputObject, ObjectBytes};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum DefinitionOwner {
    Scoop(LinkDefinitionOwnerV1),
    Runtime(RuntimeObjectId),
    Native(NativeObjectId),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ObjectOrigin {
    Cone {
        cone: ConeIdentity,
        member: SlibMemberId,
    },
    Native(NativeObjectId),
}
impl std::fmt::Display for ObjectOrigin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cone { cone, member } => write!(f, "cone-{cone}-{member}"),
            Self::Native(id) => write!(f, "native-{id}"),
        }
    }
}

pub(crate) struct ProgramInputs<'a> {
    pub objects: Vec<InputObject<'a>>,
    pub runtime_position: usize,
    pub definitions: BTreeMap<String, DefinitionOwner>,
    pub requirements: BTreeSet<String>,
    pub requirement_origins: BTreeMap<String, Vec<String>>,
    pub namespace: crate::namespace::NativeNamespace,
    pub target: scoop_lir::LirTargetProfile,
    pub target_os: scoop_target_data::TargetOs,
    pub images: Vec<String>,
    pub final_images: Vec<image::SelectedImage>,
    pub selected: SelectedMembers,
    pub root: String,
    pub string_target: String,
    pub native: NativeInputs,
    pub native_outputs: Vec<scoop_toolchain::NativeLibraryOutput>,
    pub windows_subsystem: scoop_identity::WindowsSubsystem,
}

impl<'a> ProgramInputs<'a> {
    pub fn new(
        closure: &'a ProgramLinkClosure,
        runtime: &'a RuntimeObjectSet,
        profile: &ValidatedFinalLinkProfile,
        library_paths: &[std::path::PathBuf],
        outputs: &[scoop_toolchain::NativeLibraryOutput],
    ) -> Result<Self, LinkError> {
        if (closure.root_kind() == scoop_slib::ConeKind::NativeApp)
            != (profile.output_kind() == scoop_toolchain::FinalOutputKind::SharedLibrary)
        {
            return Err(error(
                scoop_source::DiagnosticCode::E2404,
                "root Cone kind differs from final-link output kind",
            ));
        }
        if runtime.target() != profile.target() {
            return Err(error(
                scoop_source::DiagnosticCode::E2404,
                "runtime target differs from final-link target",
            ));
        }
        let target_os = scoop_target_data::definition(profile.target().id())
            .filter(|definition| definition.selection.target() == profile.target())
            .expect("validated final-link target")
            .os;
        let (declarations, libraries) = native::declarations::read(closure)?;
        let selected_names: BTreeSet<_> = libraries
            .values()
            .map(|(key, _)| key.library().as_str())
            .collect();
        let source_names: BTreeSet<_> = outputs.iter().map(|output| output.name.as_str()).collect();
        let mut outputs = outputs.to_vec();
        let mut descriptors = Vec::new();
        if !selected_names.is_empty() {
            for root in library_paths {
                descriptors.push(root.join("native-outputs.json"));
                if let Some(entries) = scoop_toolchain::native_output::read_native_outputs(root)? {
                    outputs.extend(entries.into_iter().filter(|output| {
                        selected_names.contains(output.name.as_str())
                            && !source_names.contains(output.name.as_str())
                    }));
                }
            }
        }
        let mut roots = library_paths.to_vec();
        for output in &outputs {
            if let Some(parent) = output.library.parent() {
                roots.push(parent.to_owned());
            }
            roots.extend(
                output
                    .dependencies
                    .iter()
                    .filter_map(|p| p.library.parent().map(std::path::Path::to_owned)),
            );
        }
        roots.sort();
        roots.dedup();
        let library_paths = roots.as_slice();
        let mut native = NativeInputs::read(libraries, library_paths, profile, &outputs)?;
        native.observed_paths.extend(descriptors);
        native.include_embedded(closure, profile)?;
        let namespace =
            crate::namespace::NativeNamespace::read(&mut native, library_paths, profile)?;
        // These indexes were already read for native symbol selection. Candidate
        // archive references conservatively retain Scoop entries without pulling
        // the native members themselves into the final link.
        let native_references = native
            .files
            .values()
            .flat_map(|file| file.objects())
            .flat_map(|(_, index, _, _)| &index.requirements)
            .chain(
                runtime
                    .objects()
                    .iter()
                    .flat_map(|object| &object.info().requirements),
            )
            .map(String::as_str);
        let selected = SelectedMembers::new(closure, native_references)?;
        let mut final_images = Vec::new();
        let mut objects = Vec::new();
        let mut definitions = BTreeMap::new();
        let mut requirements = BTreeSet::new();
        let mut requirement_origins: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut images = Vec::new();
        let mut root = None;
        let mut windows_subsystem = scoop_identity::WindowsSubsystem::Console;
        let mut string_target = None;
        for (artifact, symbols) in closure.artifacts() {
            let cone = artifact.identity();
            let image = image::SelectedImage::new(artifact, selected.definitions(cone), profile);
            images.push(image.name.clone());
            requirements.extend(image.references().cloned());
            for name in image.defined_names() {
                let owner = symbols
                    .defined_symbols()
                    .owners()
                    .iter()
                    .find(|record| record.symbol() == name.as_bytes())
                    .ok_or_else(|| error(
                        scoop_source::DiagnosticCode::E2409,
                        format!("Cone {cone} has no definition owner for generated image symbol {name}"),
                    ))?;
                register_scoop_definition(&mut definitions, name, owner.owner(), cone)?;
            }
            final_images.push(image);
            if let Some(entry) = artifact.production().entry_symbol {
                if cone != closure.root() || root.is_some() {
                    return Err(error(
                        scoop_source::DiagnosticCode::E2412,
                        format!("multiple executable entries at Cone {cone}"),
                    ));
                }
                root = Some(object_symbol(entry, profile));
                windows_subsystem = artifact
                    .manifest()
                    .cone()
                    .windows_subsystem()
                    .unwrap_or_default();
            }
            for object in symbols.members() {
                let member = object.member();
                if !selected.contains(cone, member) {
                    continue;
                }
                let directory = artifact.manifest().members();
                let position = directory
                    .binary_search_by_key(&member, |record| record.id())
                    .map_err(|_| {
                        error(
                            scoop_source::DiagnosticCode::E2403,
                            format!("missing object member {member} in Cone {cone}"),
                        )
                    })?;
                let record = &directory[position];
                objects.push(InputObject {
                    origin: ObjectOrigin::Cone { cone, member },
                    bytes: ObjectBytes::Artifact { artifact, member },
                    digest: record.sha256(),
                });
            }
            for definition in symbols.defined_symbols().owners() {
                if !selected.contains(cone, definition.member()) {
                    continue;
                }
                let name = std::str::from_utf8(definition.symbol())
                    .map_err(|cause| error(scoop_source::DiagnosticCode::ICE2401, cause))?;
                register_scoop_definition(&mut definitions, name, definition.owner(), cone)?;
            }
            for alias in symbols.link_support().runtime_data_aliases() {
                let record = symbols
                    .defined_symbols()
                    .owners()
                    .iter()
                    .find(|record| {
                        record.owner() == LinkDefinitionOwnerV1::StrongDefinition(alias.owner())
                    })
                    .ok_or_else(|| {
                        error(
                            scoop_source::DiagnosticCode::E2409,
                            format!(
                                "Cone {cone}: runtime String alias has no physical TD definition"
                            ),
                        )
                    })?;
                let target = String::from_utf8(record.symbol().to_vec())
                    .map_err(|cause| error(scoop_source::DiagnosticCode::ICE2401, cause))?;
                if string_target.replace(target).is_some() {
                    return Err(error(
                        scoop_source::DiagnosticCode::E2412,
                        "runtime String alias has more than one provider",
                    ));
                }
            }
            for requirement in symbols.requirements() {
                if !selected.contains(cone, requirement.member()) {
                    continue;
                }
                let symbol = String::from_utf8(requirement.symbol().to_vec())
                    .map_err(|cause| error(scoop_source::DiagnosticCode::ICE2401, cause))?;
                requirement_origins
                    .entry(symbol.clone())
                    .or_default()
                    .push(format!(
                        "Cone {} member {}",
                        artifact.manifest().cone().coordinate(),
                        requirement.member()
                    ));
                requirements.insert(symbol);
            }
        }
        for object in runtime.objects() {
            for symbol in object.info().definitions.keys() {
                match definitions.entry(symbol.clone()) {
                    std::collections::btree_map::Entry::Vacant(entry) => {
                        entry.insert(DefinitionOwner::Runtime(object.id()));
                    }
                    std::collections::btree_map::Entry::Occupied(entry) => {
                        // RuntimeObjectSet already resolved coalescible definitions.
                        if !matches!(entry.get(), DefinitionOwner::Runtime(_)) {
                            return Err(error(
                                scoop_source::DiagnosticCode::E2412,
                                format!(
                                    "runtime object {} conflicts with {:?} at {symbol}",
                                    object.id(),
                                    entry.get()
                                ),
                            ));
                        }
                    }
                }
            }
            requirements.extend(object.info().requirements.iter().cloned());
            for symbol in &object.info().requirements {
                requirement_origins
                    .entry(symbol.clone())
                    .or_default()
                    .push(format!("runtime object {}", object.id()));
            }
        }
        let root = root.ok_or_else(|| {
            error(
                scoop_source::DiagnosticCode::E2308,
                "executable closure has no root entry",
            )
        })?;
        let string_target = string_target.ok_or_else(|| {
            error(
                scoop_source::DiagnosticCode::E2409,
                "executable closure has no typed runtime String alias",
            )
        })?;
        requirements.extend([
            root.clone(),
            profile
                .target()
                .contract()
                .native_symbol_normalization()
                .compiler_generated_object_symbol(
                    if profile.output_kind() == scoop_toolchain::FinalOutputKind::SharedLibrary {
                        "scoop_rt_init_native_app"
                    } else if target_os == scoop_target_data::TargetOs::Windows {
                        "scoop_rt_run_program_utf16"
                    } else {
                        "scoop_rt_run_program"
                    },
                ),
        ]);
        requirements.extend(
            profile
                .linker_system_requirements()
                .iter()
                .map(|symbol| (*symbol).to_owned()),
        );
        let mut result = Self {
            runtime_position: objects.len(),
            objects,
            definitions,
            requirements,
            requirement_origins,
            namespace,
            target: profile.target(),
            target_os,
            images,
            final_images,
            selected,
            root,
            string_target,
            native,
            native_outputs: outputs,
            windows_subsystem,
        };
        native::resolve(
            closure,
            &declarations,
            runtime,
            profile,
            library_paths,
            &mut result,
        )?;
        Ok(result)
    }

    pub fn symbol(&self, logical: &str) -> String {
        self.target
            .contract()
            .native_symbol_normalization()
            .compiler_generated_object_symbol(logical)
    }

    pub fn compiler_owned(&self, symbol: &str) -> bool {
        symbol == self.symbol("main")
            || (self.target_os == scoop_target_data::TargetOs::Windows && symbol == "wmain")
            || symbol == self.symbol("scoop_td_String")
            || symbol.starts_with(&self.symbol("scoop$"))
    }

    pub fn linker_defined(&self, symbol: &str) -> bool {
        symbol == self.symbol("scoop_td_String") || symbol == self.symbol("__dso_handle")
    }
}

/// Generated image symbols and selected object symbols occupy one linker namespace.
fn register_scoop_definition(
    definitions: &mut BTreeMap<String, DefinitionOwner>,
    name: &str,
    owner: LinkDefinitionOwnerV1,
    cone: ConeIdentity,
) -> Result<(), LinkError> {
    use std::collections::btree_map::Entry;
    let owner = DefinitionOwner::Scoop(owner);
    match definitions.entry(name.to_owned()) {
        Entry::Vacant(entry) => {
            entry.insert(owner);
            Ok(())
        }
        Entry::Occupied(entry) => Err(error(
            scoop_source::DiagnosticCode::E2412,
            format!(
                "multiple or conflicting selected owners for {name}: {:?}, {owner:?}; Cone {cone}",
                entry.get()
            ),
        )),
    }
}

fn object_symbol(request: PersistentSymbolRequest, profile: &ValidatedFinalLinkProfile) -> String {
    profile
        .target()
        .contract()
        .native_symbol_normalization()
        .compiler_generated_object_symbol(request.symbol().as_str())
}
