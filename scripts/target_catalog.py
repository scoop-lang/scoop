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
_NATIVE_RECIPE_TOOLS = {
    "Darwin": ("Darwin", "clang"),
    "LinuxGnu": ("Linux", "gcc"),
    "LinuxMusl": ("Linux", "musl-gcc"),
    "MicrosoftUcrt": ("Windows", "clang-cl"),
    "LlvmMingwMsvcrt": ("Windows", "clang"),
    "LlvmMingwUcrt": ("Windows", "clang"),
}
# The vendored library recipes cover the desktop distribution targets.
_DISTRIBUTION_TARGETS = {target for host in HOSTS for target in host["distribution_targets"]}
NATIVE_TARGETS = {
    name: _NATIVE_RECIPE_TOOLS[target["native_environment"]]
    for name, target in TARGETS.items()
    if name in _DISTRIBUTION_TARGETS and target["os"] != "ios"
}
WINDOWS_TARGETS = tuple(name for name, target in TARGETS.items() if target["os"] == "windows")
_LLVM_COMPONENT_BY_ARCH = {"aarch64": "AArch64", "x86_64": "X86"}
_LLVM_MINGW_RESOURCE_TRIPLE_BY_ARCH = {"x86_64": "x86_64-w64-mingw32"}


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


def llvm_mingw_resource_triple(architecture):
    try:
        return _LLVM_MINGW_RESOURCE_TRIPLE_BY_ARCH[architecture]
    except KeyError as error:
        raise ValueError(
            f"no LLVM-MinGW resource triple for target architecture {architecture!r}"
        ) from error


def native_configuration(target):
    return {
        key: TARGETS[target][key]
        for key in ("os", "arch", "native_environment", "llvm_triple", "native_rust_target")
    }


def host_definition(rust_target):
    return next(host for host in HOSTS if host["rust_target"] == rust_target)


def native_target_for_host(rust_target):
    matches = [
        name for name, target in TARGETS.items() if target["native_rust_target"] == rust_target
    ]
    if len(matches) != 1:
        raise ValueError(f"host {rust_target!r} does not select exactly one native target")
    name = matches[0]
    return name, native_configuration(name)


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
