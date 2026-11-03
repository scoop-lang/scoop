use scoop_process::CommandExt;
use std::collections::BTreeSet;
use std::path::Path;

use scoop_toolchain::ValidatedFinalLinkProfile;

use crate::{LinkError, NativeObjectInfo, NativeSymbolKind, error, program::ProgramInputs};

mod images;

pub(crate) struct StartupObject {
    pub source: String,
    pub bytes: Vec<u8>,
    pub references: BTreeSet<String>,
}

fn startup_section(target: scoop_lir::LirTargetProfile) -> &'static str {
    match target.native_object_format() {
        scoop_lir::NativeObjectFormat::MachO64 => "__DATA_CONST,__const",
        scoop_lir::NativeObjectFormat::Elf64 => ".data.rel.ro.scoop.startup",
        scoop_lir::NativeObjectFormat::Coff64 => ".rdata$scoop_startup",
    }
}

impl StartupObject {
    pub fn build(
        inputs: &ProgramInputs<'_>,
        profile: &ValidatedFinalLinkProfile,
        enable_stack_trace: bool,
        directory: &Path,
    ) -> Result<Self, LinkError> {
        let mut source = String::from("#include <stddef.h>\n#include <stdint.h>\n");
        source.push_str(images::TYPES);
        let section = startup_section(profile.target());
        images::emit(&mut source, &inputs.final_images, section);
        Self::build_source(
            source,
            inputs.images.len(),
            &inputs.root,
            inputs
                .final_images
                .iter()
                .flat_map(|image| image.references().cloned())
                .collect(),
            inputs
                .final_images
                .iter()
                .flat_map(|image| {
                    std::iter::once(image.name.clone())
                        .chain(image.traps.iter().map(|(name, _)| name.clone()))
                })
                .collect(),
            profile,
            enable_stack_trace,
            directory,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn build_source(
        mut source: String,
        image_count: usize,
        root: &str,
        mut references: BTreeSet<String>,
        definitions: BTreeSet<String>,
        profile: &ValidatedFinalLinkProfile,
        enable_stack_trace: bool,
        directory: &Path,
    ) -> Result<Self, LinkError> {
        let shared = profile.output_kind() == scoop_toolchain::FinalOutputKind::SharedLibrary;
        let descriptor = if shared {
            "ScoopNativeAppEntryDescriptorV1"
        } else {
            "ScoopRootEntryDescriptorV1"
        };
        source.push_str(&format!("extern const {descriptor} root_entry __asm__(\"{root}\") __attribute__((visibility(\"hidden\")));\n"));
        let section = startup_section(profile.target());
        let windows = matches!(profile, ValidatedFinalLinkProfile::Windows(_));
        let (entry, runtime, argument_type) = if shared {
            ("scoop_app_init", "scoop_rt_init_native_app", "void")
        } else if windows {
            ("wmain", "scoop_rt_run_program_utf16", "uint16_t")
        } else {
            ("main", "scoop_rt_run_program", "char")
        };
        if shared {
            source.push_str(&format!("extern int32_t {runtime}(const ScoopImageDescriptorV1 *const *, uint64_t, const {descriptor} *, const void *, void *, ScoopAppError *, uint32_t);\n"));
        } else {
            source.push_str(&format!("extern int {runtime}(const ScoopImageDescriptorV1 *const *, uint64_t, const {descriptor} *, int32_t, const {argument_type} *const *, uint32_t);\n"));
        }
        source.push_str(&format!("\n__attribute__((used, section(\"{section}\")))\nstatic const ScoopImageDescriptorV1 *const scoop_program_images[] = {{\n"));
        for index in 0..image_count {
            source.push_str(&format!("    &image_{index},\n"));
        }
        source.push_str("};\n\n");
        if shared {
            source.push_str(&format!("__attribute__((visibility(\"default\"))) int32_t {entry}(const void *host_api, void *out_app_api, ScoopAppError *out_error) {{\n    return {runtime}(scoop_program_images, sizeof(scoop_program_images) / sizeof(scoop_program_images[0]), &root_entry, host_api, out_app_api, out_error, {});\n}}\n", u32::from(enable_stack_trace)));
        } else {
            source.push_str(&format!("int {entry}(int argc, {argument_type} **argv) {{\n    return {runtime}(scoop_program_images, sizeof(scoop_program_images) / sizeof(scoop_program_images[0]), &root_entry, argc, (const {argument_type} *const *)argv, {});\n}}\n", u32::from(enable_stack_trace)));
        }
        let source_path = directory.join("startup.c");
        let object = directory.join("startup-compiled.o");
        std::fs::write(&source_path, &source).map_err(error)?;
        let output = profile
            .startup_toolchain()
            .object_compilation_command(&source_path, &object)
            .scoop_output()
            .map_err(error)?;
        if !output.status.success() {
            return Err(error(format!(
                "startup C compilation failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }
        let bytes = std::fs::read(object).map_err(error)?;
        let mut info =
            NativeObjectInfo::read_with_toolchain(&bytes, profile.startup_toolchain().profile())?;
        if profile.target().native_object_format() == scoop_lir::NativeObjectFormat::Elf64 {
            // GNU as may retain this implicit linker symbol without a use.
            info.requirements.remove("_GLOBAL_OFFSET_TABLE_");
        }
        let normalization = profile.target().contract().native_symbol_normalization();
        let main = normalization.compiler_generated_object_symbol(entry);
        references.extend([
            root.to_owned(),
            normalization.compiler_generated_object_symbol(runtime),
        ]);
        if info.requirements != references
            || info.definitions.len() != 1 + definitions.len()
            || definitions.iter().any(|name| {
                !info.definitions.get(name).is_some_and(|definition| {
                    definition.kind == NativeSymbolKind::Data
                        && definition.binding == crate::NativeSymbolBinding::Strong
                })
            })
            || !info.definitions.get(&main).is_some_and(|definition| {
                definition.kind == NativeSymbolKind::Function
                    && definition.binding == crate::NativeSymbolBinding::Strong
            })
        {
            return Err(error(
                "startup object definitions or image/root/runtime references differ from its plan",
            ));
        }
        Ok(Self {
            source,
            bytes,
            references: info.requirements,
        })
    }
}

#[cfg(all(test, target_os = "linux", target_arch = "x86_64"))]
mod tests;
