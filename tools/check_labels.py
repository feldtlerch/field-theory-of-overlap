#!/usr/bin/env python3
"""Check the two label invariants over every Markdown file: each label is
minted exactly once, and every citation resolves to a mint. A clean
re-implementation of two invariants; the published lint tooling replaces it
when available (ADR 008). Run with --self-test to prove the harvest on a
fixture, which CI does before the real run."""
import pathlib
import re
import sys

LABEL = r"[a-z0-9-]+:[a-z0-9-]+:[a-z0-9-]+"
MINT = re.compile(rf"· `({LABEL})`")
PAREN = re.compile(r"\(([^()]*)\)")
TICKED = re.compile(rf"`({LABEL})`")


def harvest(text: str) -> tuple[list[str], list[str]]:
    mints = MINT.findall(text)
    cites = [
        label
        for group in PAREN.findall(text)
        for label in TICKED.findall(group)
    ]
    return mints, cites


def self_test() -> None:
    fixture = "# T · `rec:a:b`\n\nSee (`rec:a:b`, `dec:c:d` and `arg:e:f`).\n"
    mints, cites = harvest(fixture)
    assert mints == ["rec:a:b"], mints
    assert cites == ["rec:a:b", "dec:c:d", "arg:e:f"], cites
    print("self-test: ok")


if "--self-test" in sys.argv[1:]:
    self_test()
    sys.exit(0)

mints: dict[str, pathlib.Path] = {}
cites: list[tuple[pathlib.Path, str]] = []
failed = False
for path in sorted(pathlib.Path(".").rglob("*.md")):
    file_mints, file_cites = harvest(path.read_text(encoding="utf-8"))
    for label in file_mints:
        if label in mints:
            print(f"duplicate mint: {label} in {path} and {mints[label]}")
            failed = True
        mints[label] = path
    cites.extend((path, label) for label in file_cites)
for path, label in cites:
    if label not in mints:
        print(f"unresolved citation: {label} in {path}")
        failed = True
print(f"labels: {len(mints)} mints, {len(cites)} citations")
sys.exit(1 if failed else 0)
