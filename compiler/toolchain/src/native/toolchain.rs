use std::process::Command;

use scoop_manifest::NativeSourceLanguage;

use super::*;
use crate::ValidatedCxxToolchain;
use crate::native_driver::{NativeDriver, NativeDriverKind};

pub struct NativeToolchain {
    c: ValidatedCBridgeToolchainInvocation,
    cxx: Option<ValidatedCxxToolchain>,
    windows_c: Option<NativeDriver>,
}

impl NativeToolchain {
    pub fn prepare(
        &self,
        manifest: &LoadedConeManifest,
        optimization: OptimizationMode,
        sysroot: &Path,
        scratch: &Path,
    ) -> Result<PreparedNativeInputs, ToolchainError> {
        prepare::prepare(
            manifest,
            self.c.profile().contract().target().id(),
            self,
            optimization,
            sysroot,
            scratch,
        )
    }

    pub fn resolve(
        c: &ValidatedCBridgeToolchainInvocation,
        manifest: &LoadedConeManifest,
    ) -> Result<Self, ToolchainError> {
        Self::resolve_with(c, manifest, None, None)
    }

    pub fn resolve_target(
        target: &crate::ResolvedTargetProfile,
        manifest: &LoadedConeManifest,
    ) -> Result<Self, ToolchainError> {
        Self::resolve_with(
            target.c_bridge_toolchain(),
            manifest,
            target.native_c_driver(),
            target.native_cxx_driver(),
        )
    }

    pub fn resolve_with(
        c: &ValidatedCBridgeToolchainInvocation,
        manifest: &LoadedConeManifest,
        selected_c: Option<&Path>,
        selected_cxx: Option<&Path>,
    ) -> Result<Self, ToolchainError> {
        let cxx = manifest
            .parsed()
            .semantic()
            .native()
            .cxx(c.profile().contract().target().id());
        let windows_c = Self::needs_windows_c(c, manifest)?
            .then(|| {
                let default =
                    crate::windows_native::paired_driver(c, NativeSourceLanguage::C, selected_c);
                NativeDriver::from_discovery(crate::windows_native::discover(
                    c,
                    NativeSourceLanguage::C,
                    &default,
                )?)
            })
            .transpose()?;
        let paired_cxx = crate::windows_native::is_windows_target(c).then(|| {
            crate::windows_native::paired_driver(c, NativeSourceLanguage::Cxx, selected_c)
        });
        Ok(Self {
            c: c.clone(),
            windows_c,
            cxx: cxx
                .then(|| {
                    ValidatedCxxToolchain::resolve_with(c, selected_cxx.or(paired_cxx.as_deref()))
                })
                .transpose()
                .map_err(|err| {
                    ToolchainError(format!("{err}; C++ required by {}", manifest.coordinate()))
                })?,
        })
    }

    pub(super) fn kind(&self, language: NativeSourceLanguage) -> NativeDriverKind {
        match language {
            NativeSourceLanguage::ObjectiveC => NativeDriverKind::Gnu,
            NativeSourceLanguage::C => self
                .windows_c
                .as_ref()
                .map_or(NativeDriverKind::Gnu, NativeDriver::kind),
            NativeSourceLanguage::Cxx => self
                .cxx
                .as_ref()
                .map_or(NativeDriverKind::Gnu, ValidatedCxxToolchain::kind),
        }
    }

    fn needs_windows_c(
        c: &ValidatedCBridgeToolchainInvocation,
        manifest: &LoadedConeManifest,
    ) -> Result<bool, ToolchainError> {
        if !crate::windows_native::is_windows_target(c) {
            return Ok(false);
        }
        let config = manifest.parsed().semantic().native();
        let target_facts = crate::manifest::source_target(c.target().id())?;
        for source in config
            .sources()
            .iter()
            .filter(|s| s.predicate().matches(c.target().id()))
        {
            if config
                .source_language(source.path(), target_facts)
                .map_err(error)?
                == NativeSourceLanguage::C
            {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub(super) fn c(&self) -> &ValidatedCBridgeToolchainInvocation {
        &self.c
    }

    pub(super) fn command(
        &self,
        language: NativeSourceLanguage,
    ) -> Result<Command, ToolchainError> {
        match language {
            NativeSourceLanguage::C => Ok(self.windows_c.as_ref().map_or_else(
                || self.c.driver_command(),
                |driver| driver.command(&self.c, language),
            )),
            NativeSourceLanguage::ObjectiveC => Ok(self.c.driver_command()),
            NativeSourceLanguage::Cxx => self
                .cxx
                .as_ref()
                .map(|cxx| cxx.command(&self.c))
                .ok_or_else(|| error("C++ translation unit requires native.cxx = true")),
        }
    }
}
