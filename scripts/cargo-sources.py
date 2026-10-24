#!/usr/bin/env python3
"""Write flatpak/cargo-sources.json from Cargo.lock.

The Flatpak builds offline: every crate in Cargo.lock is fetched as an
archive the manifest names, checked against the lockfile's own sha256,
and vendored. This file is that list, derived from Cargo.lock alone —
run it after any change to the lockfile:

    scripts/cargo-sources.py            # Cargo.lock -> flatpak/cargo-sources.json
    scripts/cargo-sources.py --check    # exit 1 if the file is out of date

Same output as flatpak-builder-tools' flatpak-cargo-generator for a
lockfile of crates.io packages only, with nothing but the standard
library and no network.
"""
import json
import sys
import tomllib
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
LOCK = REPO / "Cargo.lock"
OUT = REPO / "flatpak" / "cargo-sources.json"
CRATES_IO = "registry+https://github.com/rust-lang/crates.io-index"


def sources(lock: dict) -> list:
    out = []
    for package in lock["package"]:
        source = package.get("source")
        if source is None:  # this workspace's own packages
            continue
        if source != CRATES_IO:
            # A git or other registry dependency needs a source of its own
            # kind; skipping it would leave the offline build short of it.
            raise SystemExit(f"{package['name']} {package['version']}: {source} is not crates.io;"
                             " use flatpak-cargo-generator for this lockfile")
        name, version, checksum = package["name"], package["version"], package["checksum"]
        dest = f"cargo/vendor/{name}-{version}"
        out.append({
            "type": "archive",
            "archive-type": "tar-gzip",
            "url": f"https://static.crates.io/crates/{name}/{name}-{version}.crate",
            "sha256": checksum,
            "dest": dest,
        })
        out.append({
            "type": "inline",
            "contents": json.dumps({"package": checksum, "files": {}}),
            "dest": dest,
            "dest-filename": ".cargo-checksum.json",
        })
    out.append({
        "type": "inline",
        "contents": '[source.vendored-sources]\ndirectory = "cargo/vendor"\n\n'
                    '[source.crates-io]\nreplace-with = "vendored-sources"\n',
        "dest": "cargo",
        "dest-filename": "config",
    })
    return out


def main() -> int:
    text = json.dumps(sources(tomllib.loads(LOCK.read_text())), indent=4)
    if "--check" in sys.argv[1:]:
        if OUT.read_text().rstrip("\n") != text:
            print(f"{OUT.relative_to(REPO)} is out of date: run scripts/cargo-sources.py", file=sys.stderr)
            return 1
        return 0
    OUT.write_text(text)
    return 0


if __name__ == "__main__":
    sys.exit(main())
