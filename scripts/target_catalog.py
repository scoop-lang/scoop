"""Static target and host facts shared with the compiler, without tool discovery."""

import platform
import tomllib
from pathlib import Path

CATALOG_PATH = Path(__file__).resolve().parent.parent / "compiler/target-data/targets.toml"
with CATALOG_PATH.open("rb") as source:
    _catalog = tomllib.load(source)

TARGETS = {target["canonical"]: target for target in _catalog["targets"]}
HOSTS = _catalog["hosts"]
TARGET_FACTS = {
    name: {key: target[key] for key in ("os", "arch", "env")} for name, target in TARGETS.items()
}
SYSTEMS = {"darwin": "Darwin", "linux": "Linux", "windows": "Windows"}
DEFAULT_CC = {
    "Darwin": "clang",
    "LinuxGnu": "gcc",
    "LinuxMusl": "musl-gcc",
    "MicrosoftUcrt": "clang-cl",
    "LlvmMingwMsvcrt": "clang",
    "LlvmMingwUcrt": "clang",
}
NATIVE_TARGETS = {
    name: (SYSTEMS[target["os"]], DEFAULT_CC[target["native_environment"]])
    for name, target in TARGETS.items()
}
WINDOWS_TARGETS = tuple(name for name, target in TARGETS.items() if target["os"] == "windows")
_LLVM_COMPONENT_BY_ARCH = {"aarch64": "AArch64", "x86_64": "X86"}


def llvm_target_components(targets=None):
    """Return LLVM build components for the architectures used by current targets."""
    targets = TARGETS.values() if targets is None else targets
    components = []
    for target in targets:
        architecture = target["arch"]
        try:
            component = _LLVM_COMPONENT_BY_ARCH[architecture]
        except KeyError as error:
            raise ValueError(
                f"no LLVM component for target architecture {architecture!r}"
            ) from error
        if component not in components:
            components.append(component)
    return tuple(components)


def native_configuration(target):
    return {
        key: TARGETS[target][key]
        for key in ("os", "arch", "native_environment", "llvm_triple", "native_rust_target")
    }


def host_definition(rust_target):
    return next(host for host in HOSTS if host["rust_target"] == rust_target)


def current_host(system=None, machine=None, environment=""):
    system = system or platform.system()
    machine = (machine or platform.machine()).lower()
    return next(
        (
            host
            for host in HOSTS
            if host["system"] == system
            and machine in host["machines"]
            and host["environment"] == environment
        ),
        None,
    )
