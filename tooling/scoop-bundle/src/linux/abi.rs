use std::path::Path;

use anyhow::{Result, bail, ensure};
use goblin::elf::{Elf, header};

use crate::target::Target;

pub(super) struct LinuxAbi {
    machine: u16,
    pub deb_arch: &'static str,
    pub ldconfig_arch: &'static str,
    pub interpreter: &'static str,
    musl_soname: Option<&'static str>,
}

impl LinuxAbi {
    pub fn for_target(target: &Target) -> Result<Self> {
        let (machine, deb_arch, ldconfig_arch, gnu_loader, musl_loader, musl_soname) =
            match target.arch.as_str() {
                "aarch64" => (
                    header::EM_AARCH64,
                    "arm64",
                    "AArch64",
                    "/lib/ld-linux-aarch64.so.1",
                    "/lib/ld-musl-aarch64.so.1",
                    "libc.musl-aarch64.so.1",
                ),
                "x86_64" => (
                    header::EM_X86_64,
                    "amd64",
                    "x86-64",
                    "/lib64/ld-linux-x86-64.so.2",
                    "/lib/ld-musl-x86_64.so.1",
                    "libc.musl-x86_64.so.1",
                ),
                _ => bail!("unsupported Linux package architecture {}", target.arch),
            };
        let (interpreter, musl_soname) = match target.env.as_str() {
            "gnu" => (gnu_loader, None),
            "musl" => (musl_loader, Some(musl_soname)),
            _ => bail!("unsupported Linux package environment {}", target.env),
        };
        Ok(Self {
            machine,
            deb_arch,
            ldconfig_arch,
            interpreter,
            musl_soname,
        })
    }

    pub fn check_machine(&self, elf: &Elf<'_>, path: &Path) -> Result<()> {
        ensure!(
            elf.is_64 && elf.little_endian && elf.header.e_machine == self.machine,
            "{}: expected a little-endian 64-bit {} ELF image",
            path.display(),
            self.deb_arch
        );
        Ok(())
    }

    pub fn musl_libc(&self, name: &str) -> bool {
        self.musl_soname
            .is_some_and(|soname| name == "libc.so" || name == soname)
    }

    pub fn system_baseline(&self, name: &str) -> bool {
        let loader = self.interpreter.rsplit('/').next();
        loader == Some(name)
            || self.musl_libc(name)
            || (self.musl_soname.is_none()
                && matches!(
                    name,
                    "libc.so.6"
                        | "libm.so.6"
                        | "libpthread.so.0"
                        | "libdl.so.2"
                        | "librt.so.1"
                        | "libresolv.so.2"
                        | "libutil.so.1"
                ))
    }
}
