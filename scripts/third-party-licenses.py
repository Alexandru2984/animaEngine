#!/usr/bin/env python3
"""Write data/THIRD-PARTY-LICENSES.txt: the licences of the crates built
into animaEngine's release binaries.

Every crate a release build links on Linux, FreeBSD and Windows (default
features, no dev- or build-dependencies) with the licence texts and
copyright notices it ships — grouped where the text is the same, each
group naming its crates. MIT, BSD, Apache-2.0, ISC, Zlib, OFL and the
others ask for this text to go with the binary: every package installs
the file, and the app shows it (Appearance -> About).

    scripts/third-party-licenses.py            # write it
    scripts/third-party-licenses.py --check    # exit 1 if out of date

Needs the crates' sources on disk (`cargo fetch`) and the standard
library only. A crate published without a licence file gets its
licence's standard text, as cargo-about gives it: MIT (chosen where a
crate offers it) and BSD-2-Clause with the crate's authors as the
copyright holders, Apache-2.0, MPL-2.0 and CC0-1.0 as another crate here
ships them. A crate under none of those fails the run, so nothing is
shipped without its text.
"""
import json
import re
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
OUT = REPO / "data" / "THIRD-PARTY-LICENSES.txt"
TARGETS = ["x86_64-unknown-linux-gnu", "x86_64-unknown-freebsd", "x86_64-pc-windows-gnu"]
# A licence file: LICENSE, LICENSE-MIT, COPYING, NOTICE, UNLICENSE, the
# fonts' OFL.txt and UFL.txt.
NAME = re.compile(r"^(licen[sc]e|copying|notice|unlicense|copyright|ofl|ufl)", re.I)
# Where a crate keeps texts for what it bundles: fonts, a vendored C library.
BUNDLED = ("fonts", "upstream")
RULE = "-" * 79


def shipped(target: str) -> list[dict]:
    """The packages a release build for `target` links: the root's normal
    dependencies, and theirs, transitively."""
    meta = json.loads(subprocess.run(
        ["cargo", "metadata", "--locked", "--format-version", "1", "--filter-platform", target],
        cwd=REPO, capture_output=True, text=True, check=True).stdout)
    packages = {p["id"]: p for p in meta["packages"]}
    nodes = {n["id"]: n for n in meta["resolve"]["nodes"]}
    root = meta["resolve"]["root"]
    seen, todo = set(), [root]
    while todo:
        node = todo.pop()
        if node in seen:
            continue
        seen.add(node)
        for dep in nodes[node]["deps"]:
            if any(kind["kind"] is None for kind in dep["dep_kinds"]):
                todo.append(dep["pkg"])
    seen.discard(root)
    return [packages[i] for i in seen]


def licence_files(package: dict) -> list[Path]:
    """The crate's own licence files, then those of what it bundles."""
    root = Path(package["manifest_path"]).parent
    files = [f for f in sorted(root.iterdir()) if f.is_file() and NAME.match(f.name)]
    if package.get("license_file"):
        named = root / package["license_file"]
        if named.is_file() and named not in files:
            files.append(named)
    for sub in BUNDLED:
        d = root / sub
        if d.is_dir():
            files += [f for f in sorted(d.iterdir())
                      if f.is_file() and (NAME.match(f.name) or (sub == "fonts" and f.suffix == ".txt"))]
    return files


MIT = """MIT License

Copyright (c) {holders}

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE."""

BSD2 = """BSD 2-Clause License

Copyright (c) {holders}

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions are met:

1. Redistributions of source code must retain the above copyright notice, this
   list of conditions and the following disclaimer.

2. Redistributions in binary form must reproduce the above copyright notice,
   this list of conditions and the following disclaimer in the documentation
   and/or other materials provided with the distribution.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE
FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE."""

# The full texts some crate here ships, told by how they open.
FULL = {
    "Apache-2.0": lambda t: t.startswith("Apache License") and "Version 2.0, January 2004" in t[:200]
    and "END OF TERMS AND CONDITIONS" in t,
    "MPL-2.0": lambda t: t.startswith("Mozilla Public License Version 2.0"),
    "CC0-1.0": lambda t: t.startswith("Creative Commons Legal Code") and "CC0 1.0 Universal" in t[:100],
}


def standard_text(package: dict, full: dict) -> str | None:
    """The standard text of a licence `package` is offered under, for a
    crate published without one."""
    offered = re.split(r"\s+OR\s+|/", package.get("license") or "")
    holders = ", ".join(re.sub(r"\s*<[^>]*>", "", a) for a in package.get("authors") or []) \
        or f"the {package['name']} authors"
    for spdx in ["MIT", "BSD-2-Clause", "Apache-2.0", "MPL-2.0", "CC0-1.0"]:
        if spdx in offered:
            if spdx == "MIT":
                return MIT.format(holders=holders)
            if spdx == "BSD-2-Clause":
                return BSD2.format(holders=holders)
            if spdx in full:
                return full[spdx]
    return None


COPYRIGHT = re.compile(r"^\s*(copyright\b|\(c\)|©)", re.I)
MIT_TITLE = re.compile(r"^\s*(the\s+)?mit\s+licen[sc]e(\s*\(mit\))?\s*$", re.I)
END_OF_TERMS = "END OF TERMS AND CONDITIONS"


def words(text: str) -> str:
    return " ".join(text.split())


def notices(lines: list[str]) -> list[str]:
    """Its copyright lines, less a template's ("[yyyy] [name of copyright
    owner]")."""
    return [" ".join(l.split()) for l in lines if COPYRIGHT.match(l) and "[yyyy]" not in l and "{yyyy}" not in l]


def mit_notices(text: str) -> list[str] | None:
    """The copyright lines of an MIT licence with the standard terms, or
    None for any other text."""
    lines = text.splitlines()
    body = [l for l in lines if not COPYRIGHT.match(l) and not MIT_TITLE.match(l)]
    standard = [l for l in MIT.splitlines() if not COPYRIGHT.match(l) and not MIT_TITLE.match(l)]
    return notices(lines) if words("\n".join(body)) == words("\n".join(standard)) else None


START_OF_TERMS = "TERMS AND CONDITIONS FOR USE, REPRODUCTION, AND DISTRIBUTION"


def apache_core(text: str) -> str | None:
    """An Apache-2.0 text's terms, from their heading to their end, as
    words — list markers and the URL's scheme aside, which copies vary
    in — or None for a text that is not one."""
    if not text.startswith("Apache License") or START_OF_TERMS not in text or END_OF_TERMS not in text:
        return None
    core = text.partition(START_OF_TERMS)[2].partition(END_OF_TERMS)[0]
    core = re.sub(r"\(\w\)", " ", core.replace("https://", "http://"))
    return words(core)


def apache_notices(text: str, terms: str) -> list[str] | None:
    """The copyright lines of an Apache-2.0 licence with the standard terms
    — what follows them is the appendix, how to apply it — or None."""
    if apache_core(text) != apache_core(terms):
        return None
    return notices(text.partition(END_OF_TERMS)[2].splitlines())


def text_of(path: Path) -> str:
    raw = path.read_text(encoding="utf-8", errors="replace")
    return "\n".join(line.rstrip() for line in raw.strip().splitlines())


def build() -> str:
    crates = {}
    for target in TARGETS:
        for p in shipped(target):
            crates[(p["name"], p["version"])] = p
    groups: dict[str, list[str]] = {}
    texts = {key: [text_of(f) for f in licence_files(p)] for key, p in sorted(crates.items())}
    full = {}
    for spdx, looks in FULL.items():
        found = sorted(t for ts in texts.values() for t in ts if looks(t))
        if found:
            full[spdx] = found[0]
    # The Apache terms most of them ship, as one of them words them.
    apaches = sorted(x for xs in texts.values() for x in xs if apache_core(x))
    common = max(set(map(apache_core, apaches)), key=lambda c: (sum(apache_core(x) == c for x in apaches), c))
    terms = next(x for x in apaches if apache_core(x) == common)
    terms = terms.partition(END_OF_TERMS)[0].rstrip() + "\n\n" + END_OF_TERMS
    # A crate published without its text gets the terms alone: the
    # appendix of the copy they came from names that crate's authors.
    full["Apache-2.0"] = terms
    MIT_SHARED = MIT.format(holders="the holders named above, each for their crate")
    APACHE_SHARED = terms
    missing = []
    for (name, version), p in sorted(crates.items()):
        label = f"{name} {version} ({p.get('license') or 'see text'})"
        own = texts[(name, version)]
        if not own:
            standard = standard_text(p, full)
            if standard is None:
                missing.append(f"{name} {version} ({p.get('license')})")
                continue
            own = [standard]
            label += " - published without a licence file: the standard text"
        offered = re.split(r"\s+OR\s+|/", p.get("license") or "")
        mits = [m for m in (mit_notices(x) for x in own) if m is not None]
        if "MIT" in offered and mits:
            # Under MIT, of the licences offered: its Apache text goes.
            own = [x for x in own if mit_notices(x) is not None or apache_notices(x, terms) is None]
        for text in own:
            # One copy of the MIT and of the Apache terms, each crate under
            # it named with its copyright lines.
            shared = mit_notices(text)
            key = shared is not None and MIT_SHARED
            if shared is None:
                shared = apache_notices(text, terms)
                key = shared is not None and APACHE_SHARED
            if key:
                groups.setdefault(key, []).append(label + ("" if not shared else ": " + "; ".join(shared)))
            else:
                groups.setdefault(text, []).append(label)
    if missing:
        sys.exit("no licence file in: " + ", ".join(missing))
    head = f"""Third-party software in animaEngine
===================================

animaEngine is under the MIT licence (LICENSE). Its release binaries are
built with the {len(crates)} open-source Rust crates below, among them egui's
fonts (epaint_default_fonts) and the OpenH264 decoder (openh264-sys2).
Their licence texts and copyright notices follow: one section per
distinct text, headed by the crates it comes with, the licence each is
offered under and, where the text is the standard MIT or Apache-2.0 one
shared by many, each crate's own copyright lines. A crate offered under
MIT or another licence ("MIT OR Apache-2.0") is used here under MIT.

Each crate's source is at https://crates.io/crates/NAME/VERSION — for the
MPL-2.0 ones, that is where their Source Code Form is available.

Written by scripts/third-party-licenses.py from Cargo.lock.
"""
    parts = [head]
    for text, labels in sorted(groups.items(), key=lambda g: g[1][0]):
        parts.append(f"{RULE}\n" + "\n".join(labels) + f"\n{RULE}\n\n{text}\n")
    return "\n".join(parts)


def main() -> int:
    text = build()
    if "--check" in sys.argv[1:]:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            print(f"{OUT.relative_to(REPO)} is out of date: run scripts/third-party-licenses.py", file=sys.stderr)
            return 1
        return 0
    OUT.write_text(text, encoding="utf-8")
    return 0


if __name__ == "__main__":
    sys.exit(main())
