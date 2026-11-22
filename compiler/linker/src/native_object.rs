//! Symbol information used to select ordinary native objects.
use std::collections::{BTreeMap, BTreeSet};

use object::{
    Object, ObjectKind, ObjectSection, ObjectSymbol, SymbolKind, macho, read::macho::MachOFile64,
};
use scoop_lir::{CBridgeToolchainProfileV1, DarwinCBridgeDeploymentContractV1};
use scoop_target_data::ApplePlatform;

use crate::{LinkError, error};

pub(crate) mod coff;
mod elf;
mod wire;

#[cfg(all(test, target_os = "macos", target_arch = "aarch64"))]
mod tests;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeSymbolKind {
    Function,
    Data,
    ThreadLocal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeSymbolBinding {
    Weak,
    Common,
    Strong,
}

impl NativeSymbolBinding {
    pub(crate) fn precedence(self, format: scoop_lir::NativeObjectFormat) -> u8 {
        match (self, format) {
            (Self::Strong, _) => 2,
            (Self::Common, scoop_lir::NativeObjectFormat::Elf64)
            | (Self::Common, scoop_lir::NativeObjectFormat::Coff64)
            | (Self::Weak, scoop_lir::NativeObjectFormat::MachO64) => 1,
            _ => 0,
        }
    }

    pub(crate) fn from_weak(weak: bool) -> Self {
        if weak { Self::Weak } else { Self::Strong }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NativeSymbolDefinition {
    pub kind: NativeSymbolKind,
    pub read_only: bool,
    pub binding: NativeSymbolBinding,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeObjectInfo {
    pub definitions: BTreeMap<String, NativeSymbolDefinition>,
    pub requirements: BTreeSet<String>,
}

impl NativeObjectInfo {
    pub fn read_with_toolchain(
        bytes: &[u8],
        toolchain: &CBridgeToolchainProfileV1,
    ) -> Result<Self, LinkError> {
        let target = toolchain.contract().target().id();
        let profile = scoop_target_data::layout(target).ok_or_else(|| {
            error(
                scoop_source::DiagnosticCode::E2301,
                "unsupported native object target",
            )
        })?;
        match profile.native_object_format() {
            scoop_lir::NativeObjectFormat::MachO64 => Self::read(
                bytes,
                target,
                toolchain
                    .contract()
                    .deployment()
                    .map_err(|cause| error(scoop_source::DiagnosticCode::ICE2201, cause))?,
            ),
            scoop_lir::NativeObjectFormat::Elf64 => elf::read(bytes, target),
            scoop_lir::NativeObjectFormat::Coff64 => coff::read(bytes, profile),
        }
    }

    pub fn read(
        bytes: &[u8],
        target: scoop_lir::TargetProfileId,
        deployment: &DarwinCBridgeDeploymentContractV1,
    ) -> Result<Self, LinkError> {
        let machine = scoop_slib::MachOArchitecture::for_target(target)
            .map_err(|cause| error(scoop_source::DiagnosticCode::E2301, cause))?;
        let platform = scoop_target_data::definition(target)
            .and_then(|definition| definition.apple_platform())
            .ok_or_else(|| {
                error(
                    scoop_source::DiagnosticCode::E2301,
                    "missing Apple deployment platform",
                )
            })?;
        let file: MachOFile64<'_> = MachOFile64::parse(bytes)
            .map_err(|cause| error(scoop_source::DiagnosticCode::E2305, cause))?;
        if file.architecture() != machine.object_architecture()
            || file.kind() != ObjectKind::Relocatable
            || !file.is_little_endian()
            || file.macho_header().cpusubtype.get(file.endian()) != machine.cpu_subtype()
        {
            return Err(error(
                scoop_source::DiagnosticCode::E2305,
                "native input must be an ordinary little-endian Mach-O object for the selected target",
            ));
        }
        check_deployment(&file, platform, deployment)?;
        let mut definitions = BTreeMap::new();
        let mut requirements = BTreeSet::new();
        for symbol in file.symbols().filter(|symbol| symbol.is_global()) {
            let name = symbol
                .name()
                .map_err(|cause| error(scoop_source::DiagnosticCode::E2305, cause))?
                .to_owned();
            // Mach-O tentative storage uses N_UNDF with a nonzero n_value.
            if symbol.is_undefined() && symbol.address() != 0 {
                definitions.insert(
                    name,
                    NativeSymbolDefinition {
                        kind: NativeSymbolKind::Data,
                        read_only: false,
                        binding: NativeSymbolBinding::Common,
                    },
                );
                continue;
            }
            if symbol.is_undefined() {
                requirements.insert(name);
                continue;
            }
            let section = symbol
                .section_index()
                .map(|index| {
                    file.section_by_index(index)
                        .map_err(|cause| error(scoop_source::DiagnosticCode::E2305, cause))
                })
                .transpose()?;
            let instructions = section.as_ref().is_some_and(|section| {
                section.macho_section().flags.get(file.endian())
                    & (macho::S_ATTR_PURE_INSTRUCTIONS | macho::S_ATTR_SOME_INSTRUCTIONS)
                    != 0
            });
            let kind = match symbol.kind() {
                SymbolKind::Text => NativeSymbolKind::Function,
                SymbolKind::Tls => NativeSymbolKind::ThreadLocal,
                SymbolKind::Unknown if instructions => NativeSymbolKind::Function,
                _ => NativeSymbolKind::Data,
            };
            let read_only = match &section {
                Some(section) => {
                    matches!(
                        section.kind(),
                        object::SectionKind::ReadOnlyData
                            | object::SectionKind::ReadOnlyString
                            | object::SectionKind::Text
                    ) || section
                        .segment_name()
                        .map_err(|cause| error(scoop_source::DiagnosticCode::E2305, cause))?
                        == Some("__DATA_CONST")
                }
                None => true,
            };
            if definitions
                .insert(
                    name.clone(),
                    NativeSymbolDefinition {
                        kind,
                        read_only,
                        binding: NativeSymbolBinding::from_weak(symbol.is_weak()),
                    },
                )
                .is_some()
            {
                return Err(error(
                    scoop_source::DiagnosticCode::E2306,
                    format!("duplicate native definition {name}"),
                ));
            }
        }
        Ok(Self {
            definitions,
            requirements,
        })
    }
}

fn check_deployment(
    file: &MachOFile64<'_>,
    target: ApplePlatform,
    deployment: &DarwinCBridgeDeploymentContractV1,
) -> Result<(), LinkError> {
    let mut commands = file
        .macho_load_commands()
        .map_err(|cause| error(scoop_source::DiagnosticCode::E2305, cause))?;
    let endian = file.endian();
    let mut has_platform = false;
    let mut has_target = false;
    while let Some(command) = commands
        .next()
        .map_err(|cause| error(scoop_source::DiagnosticCode::E2305, cause))?
    {
        if let Some((platform, minimum)) = deployment_record(&command, endian)? {
            has_platform = true;
            if platform == target.macho_platform() {
                has_target = true;
                if minimum > deployment.minimum_os().packed() {
                    return Err(error(
                        scoop_source::DiagnosticCode::E2305,
                        format!("native object has incompatible minimum {}", target.name()),
                    ));
                }
            }
        }
    }
    if has_platform && !has_target {
        return Err(error(
            scoop_source::DiagnosticCode::E2305,
            format!("native object has no compatible {} platform", target.name()),
        ));
    }
    Ok(())
}

pub(crate) fn deployment_record(
    command: &object::read::macho::LoadCommandData<'_, object::Endianness>,
    endian: object::Endianness,
) -> Result<Option<(u32, u32)>, LinkError> {
    let platform = match command.cmd() {
        macho::LC_BUILD_VERSION => {
            let build: &macho::BuildVersionCommand<_> = command
                .data()
                .map_err(|cause| error(scoop_source::DiagnosticCode::E2305, cause))?;
            return Ok(Some((build.platform.get(endian), build.minos.get(endian))));
        }
        macho::LC_VERSION_MIN_MACOSX => macho::PLATFORM_MACOS,
        macho::LC_VERSION_MIN_IPHONEOS => macho::PLATFORM_IOS,
        macho::LC_VERSION_MIN_TVOS => macho::PLATFORM_TVOS,
        macho::LC_VERSION_MIN_WATCHOS => macho::PLATFORM_WATCHOS,
        _ => return Ok(None),
    };
    let version: &macho::VersionMinCommand<_> = command
        .data()
        .map_err(|cause| error(scoop_source::DiagnosticCode::E2305, cause))?;
    Ok(Some((platform, version.version.get(endian))))
}
