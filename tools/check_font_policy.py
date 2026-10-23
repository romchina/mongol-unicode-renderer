#!/usr/bin/env python3
"""Audit the Git index: one pinned OFL font, its license, and no other fonts.

Uses only the Python standard library and Git. Checks the index rather than
untracked local files. This is an accidental-publication guard, not a DLP system
for encoded or otherwise deliberately disguised data.
"""
import hashlib
from pathlib import Path
import subprocess
import sys


FONT = "tests/fonts/NotoSansMongolian-Regular.ttf"
LICENSE = "tests/fonts/OFL.txt"
EXPECTED = {
    FONT: "caea465b44ff4a2c0ab7bc4a2da4abcb1cf10658d70c5e49f08c9e07ae589c3a",
    LICENSE: "dbaad9386f1579930e60139d1cf36913da6c45f032433798bb8bf2e5912c8467",
}
EXTENSIONS = {
    ".ttf", ".otf", ".ttc", ".otc", ".woff", ".woff2", ".eot",
    ".fon", ".fnt", ".pfa", ".pfb", ".bdf", ".pcf", ".sfd", ".dfont",
}
SIGNATURES = (b"\x00\x01\x00\x00", b"OTTO", b"ttcf", b"wOFF", b"wOF2", b"true", b"typ1")


def check(root: Path) -> list[str]:
    files = subprocess.check_output(["git", "ls-files", "-z"], cwd=root).split(b"\0")
    errors = []
    found = set()
    for raw in filter(None, files):
        name = raw.decode("utf-8")
        data = subprocess.check_output(["git", "show", ":" + name], cwd=root)
        if name in EXPECTED:
            found.add(name)
            if hashlib.sha256(data).hexdigest() != EXPECTED[name]:
                errors.append(f"Pinned font/license checksum changed: {name}")
            continue
        path = Path(name)
        is_font = path.suffix.lower() in EXTENSIONS or data.startswith(SIGNATURES)
        is_ufo = any(part.lower().endswith(".ufo") for part in path.parts)
        if is_font or is_ufo:
            errors.append(f"Unapproved font file: {name}")
    for missing in EXPECTED.keys() - found:
        errors.append(f"Missing pinned fixture/license: {missing}")
    return errors


if __name__ == "__main__":
    problems = check(Path(__file__).resolve().parents[1])
    if problems:
        print("Font policy failed:\n" + "\n".join(problems), file=sys.stderr)
        sys.exit(1)
    print("Font policy passed: only the pinned Noto OFL fixture and matching license.")
