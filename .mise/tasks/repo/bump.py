#!/usr/bin/env -S uv run --script
# MISE alias="bump"
# MISE description="Move the crate to its next patch, minor, or major version"
# /// script
# requires-python = ">=3.11"
# dependencies = ["tomlkit==0.15.2"]
# ///
"""
Moves `crate/Cargo.toml` and the `prose` entry of `Cargo.lock` to the next
version, taking the part to raise as its one argument.

`patch` raises the last part, `minor` the middle one, and `major` the first,
each zeroing the parts after it, so `0.11.0` moves to `0.11.1`, `0.12.0`,
or `1.0.0`. Both files are rewritten through `tomlkit`, which keeps every
other byte as it stood. A version other than three dot-separated numbers,
or a lockfile whose `prose` entry differs from the manifest, stops the task
before either file is written.
"""

from argparse import ArgumentParser
from pathlib  import Path
from re       import fullmatch
from tomlkit  import parse

LOCKFILE = Path("Cargo.lock")
MANIFEST = Path("crate/Cargo.toml")
PARTS    = ("major", "minor", "patch")


def bumped(version: str, part: str) -> str:
    """
    Raises the `part` of `version` by one, zeroing every part after it.
    """
    if not fullmatch(r"\d+\.\d+\.\d+", version):
        raise SystemExit(f"{MANIFEST} carries {version}, not three numbers")
    numbers = [int(number) for number in version.split(".")]
    index   = PARTS.index(part)
    return ".".join(
        str(number + 1 if place == index else 0 if place > index else number)
        for place, number in enumerate(numbers)
    )


def main():
    """
    Reads the part from the command line, then writes the next version into
    the manifest and the lockfile and prints the move.
    """
    parser = ArgumentParser(prog="mise bump", description=__doc__.split("\n\n")[0])
    parser.add_argument("part", choices=PARTS)
    part = parser.parse_args().part

    manifest = parse(MANIFEST.read_text(encoding="utf-8"))
    lockfile = parse(LOCKFILE.read_text(encoding="utf-8"))
    package  = manifest["package"]
    locked   = next(entry for entry in lockfile["package"] if entry["name"] == "prose")
    old      = package["version"]

    if locked["version"] != old:
        raise SystemExit(f"{LOCKFILE} locks prose at {locked['version']}, not {old}")

    package["version"] = locked["version"] = new = bumped(old, part)
    MANIFEST.write_text(manifest.as_string(), encoding="utf-8")
    LOCKFILE.write_text(lockfile.as_string(), encoding="utf-8")
    print(f"prose {old} → {new}")


if __name__ == "__main__":

    main()
