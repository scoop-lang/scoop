"""Automatic discovery, carrier validation, and explicit source ownership."""

import tomllib
from pathlib import Path

from .model import ConfigurationError, Fixture
from .schema import validate


def input_files(base, names):
    result = set()
    for name in names:
        target = base / name
        if not target.exists():
            raise ConfigurationError(f"missing declared input: {target}")
        if target.is_dir():
            result.update(item.resolve() for item in target.rglob("*") if item.is_file())
        else:
            result.add(target.resolve())
    return frozenset(result)


def read_criteria(locator):
    try:
        if locator.suffix == ".scoop":
            return inline(locator)
        with locator.open("rb") as stream:
            return tomllib.load(stream)
    except (tomllib.TOMLDecodeError, UnicodeDecodeError) as error:
        raise ConfigurationError(f"{locator}: invalid fixture criteria: {error}") from error


def inline(source):
    marker = b"// fixture:begin"
    with source.open("rb") as stream:
        # Most sources use a sidecar. A fixed prefix check also handles very
        # long first lines without reading the program into memory.
        if stream.read(len(marker)) != marker:
            return None
        stream.seek(0)
        # splitlines also accepts CR-only headers. Decode only criteria so
        # invalid bytes in the program remain available to the compiler.
        lines = (line for raw in stream for line in raw.splitlines())
        if next(lines, None) != marker:
            return None
        result = []
        for line in lines:
            if line == b"// fixture:end":
                return tomllib.loads("\n".join(result))
            if not line.startswith(b"//"):
                raise ConfigurationError(f"{source}: malformed inline fixture")
            try:
                result.append(line[2:].removeprefix(b" ").decode("utf-8"))
            except UnicodeDecodeError as error:
                raise ConfigurationError(f"{source}: inline criteria must be UTF-8") from error
    raise ConfigurationError(f"{source}: unterminated inline fixture")


def expectation_files(value, base, update):
    if isinstance(value, list):
        for item in value:
            expectation_files(item, base, update)
    elif isinstance(value, dict):
        for key, item in value.items():
            if key in ("file", "snapshot") and isinstance(item, str) and "${" not in item:
                if not (base / item).is_file() and not (update and key == "snapshot"):
                    raise ConfigurationError(f"missing expectation file: {base / item}")
            else:
                expectation_files(item, base, update)


def discover(suite: Path, update=False, target=None):
    sources = set(suite.rglob("*.scoop"))
    carriers = sorted(set(suite.rglob("*.fixture.toml")) | set(suite.rglob("fixture.toml")))
    raw = [(carrier, read_criteria(carrier)) for carrier in carriers]
    for source in sorted(sources):
        data = read_criteria(source)
        if data is not None:
            if source.with_suffix(".fixture.toml").exists():
                raise ConfigurationError(f"{source}: duplicate inline and sidecar criteria")
            raw.append((source, data))
    fixtures, names, covered = [], set(), set()
    for locator, data in raw:
        try:
            base = locator.parent
            validate(data, base, target)
            expectation_files(data["steps"], base, update)
            inputs = input_files(base, data["inputs"])
            support = input_files(base, data.get("support", []))
            if data["name"] in names:
                raise ConfigurationError(f"duplicate fixture name {data['name']}")
            if "root" in data and not (base / data["root"]).exists():
                raise ConfigurationError(f"missing root {data['root']}")
            if locator.suffix == ".scoop" and locator.resolve() not in inputs:
                raise ConfigurationError("inline fixture must own its source")
            fixtures.append(Fixture(locator, base, data["name"], data, inputs, support))
            names.add(data["name"])
            covered.update(inputs | support)
        except (ValueError, TypeError, KeyError) as error:
            raise ConfigurationError(f"{locator}: {error}") from error
    uncovered = {source.resolve() for source in sources} - covered
    if uncovered:
        examples = "\n".join(str(item) for item in sorted(uncovered)[:30])
        raise ConfigurationError(
            f"{len(uncovered)} source files have no declared ownership:\n{examples}"
        )
    return sorted(fixtures, key=lambda fixture: fixture.name)
