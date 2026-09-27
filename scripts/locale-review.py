#!/usr/bin/env python3
"""Write one "Locale review" issue body per locale, listing what changed.

    scripts/locale-review.py --since REF [--out DIR]

For every locale except English, lists the messages whose translation
is new or changed since REF, plus the ones whose English changed while
the translation did not. REF is usually the last reviewed release: v1.1.0
lists everything 1.2 added. Each message sits next to the English it
translates. Strings added during a release are machine-translated unless
a native speaker wrote them, so this is the list that needs a native pass.

The bodies follow .github/ISSUE_TEMPLATE/locale-review.md and land in
DIR (default build/locale-review/), one <lang>.md each, with the issue
title on the first line. Posting them is left to a maintainer.
"""

import argparse
import pathlib
import subprocess
import sys

REPO = pathlib.Path(__file__).resolve().parent.parent
LOCALES = REPO / "src" / "i18n" / "locales"

NAMES = {
    "ro": "Română",
    "de": "Deutsch",
    "es": "Español",
    "fr": "Français",
    "it": "Italiano",
    "pt-BR": "Português (Brasil)",
    "pl": "Polski",
    "nl": "Nederlands",
    "ja": "日本語",
}

# Registers a test pins (i18n::tests::each_language_keeps_one_register):
# a reviewer changing one must change the whole file, and the test.
REGISTER = {
    "de": "formal — *Sie*",
    "nl": "formal — *u*",
    "es": "informal — *tú*",
}


def parse(text):
    """Message id -> value, in file order. Values are single-line."""
    out = {}
    for line in text.splitlines():
        if not line or line.startswith("#") or line[0].isspace():
            continue
        key, sep, value = line.partition(" = ")
        if sep:
            out[key.strip()] = value.strip()
    return out


def at_ref(ref, name):
    path = f"src/i18n/locales/{name}.ftl"
    shown = subprocess.run(
        ["git", "-C", str(REPO), "show", f"{ref}:{path}"],
        capture_output=True, text=True,
    )
    return parse(shown.stdout) if shown.returncode == 0 else {}


def blob_base():
    """Absolute links: an issue resolves relative ones against its own URL."""
    url = subprocess.run(
        ["git", "-C", str(REPO), "remote", "get-url", "origin"],
        capture_output=True, text=True,
    ).stdout.strip()
    url = url.removesuffix(".git").replace("git@github.com:", "https://github.com/")
    return f"{url}/blob/main" if url.startswith("https://github.com/") else ".."


def cell(value):
    return value.replace("|", "\\|") if value else "—"


def body(lang, since, rows, base):
    reg = REGISTER.get(lang)
    lines = [
        f"i18n({lang}): native-speaker pass — strings since {since}",
        "",
        "## Locale",
        "",
        f"`{lang}` — {NAMES.get(lang, lang)}",
        "",
        "## What needs review",
        "",
        "- [ ] First-time review",
        "- [x] Drift since the last audit (new strings have landed in",
        "      English and need translating)",
        "- [ ] Suspected mistranslation",
        "- [ ] Glossary inconsistency",
        "- [ ] Register / tone",
        "",
        f"The {len(rows)} messages below are new or changed since `{since}`.",
        "Their translations were machine-made and have not been read by a",
        "native speaker. Anything else in the file is out of scope here.",
        "",
        "For each row: leave *Suggested fix* empty if the translation reads",
        "well, or write the text you would use. Keep every `{ $name }`",
        "placeholder exactly as it is — the tests fail otherwise.",
    ]
    if reg:
        lines += [
            "",
            f"This file addresses the user in one register, {reg}; a test",
            "holds it to that, so please keep to it.",
        ]
    lines += [
        "",
        "## Reference material",
        "",
        f"- Pipeline + workflow: [`docs/i18n-pipeline.md`]({base}/docs/i18n-pipeline.md)",
        f"- Current audit doc: [`docs/locale-audit/{lang}.md`]({base}/docs/locale-audit/{lang}.md)",
        f"- Canonical English: [`src/i18n/locales/en.ftl`]({base}/src/i18n/locales/en.ftl)",
        f"- Source file: [`src/i18n/locales/{lang}.ftl`]({base}/src/i18n/locales/{lang}.ftl)",
        "",
        "## Specific keys / sections",
        "",
        "| Message id | English | Current value | Suggested fix | Reason |",
        "|---|---|---|---|---|",
    ]
    for key, en, cur, reason in rows:
        lines.append(f"| `{key}` | {cell(en)} | {cell(cur)} |  | {reason} |")
    lines += [
        "",
        "## Reviewer",
        "",
        "<!-- Optional: GitHub handle or contact. Drive-by review is welcome. -->",
        "",
        "## Acceptance",
        "",
        "Once a PR opens with this work:",
        "",
        "- [ ] `cargo test --lib i18n` passes (every-locale-covers-every-en-key",
        "      + every-locale-parses)",
        "- [ ] Audit doc updated to reflect what was applied / deferred /",
        "      rejected (don't silently drop suggestions)",
        "",
    ]
    return "\n".join(lines)


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--since", required=True, help="git ref to compare against, e.g. v1.1.0")
    ap.add_argument("--out", default=str(REPO / "build" / "locale-review"))
    args = ap.parse_args()
    since = args.since

    en_now = parse((LOCALES / "en.ftl").read_text(encoding="utf-8"))
    en_then = at_ref(since, "en")
    base = blob_base()
    out = pathlib.Path(args.out)
    out.mkdir(parents=True, exist_ok=True)

    for lang in NAMES:
        now = parse((LOCALES / f"{lang}.ftl").read_text(encoding="utf-8"))
        then = at_ref(since, lang)
        rows = []
        for key, value in now.items():
            en = en_now.get(key, "")
            if key not in then:
                rows.append((key, en, value, "new"))
            elif then[key] != value:
                rows.append((key, en, value, "changed"))
            elif en_then.get(key) not in (None, en):
                rows.append((key, en, value, "English changed; this did not"))
        target = out / f"{lang}.md"
        target.write_text(body(lang, since, rows, base), encoding="utf-8")
        print(f"{lang:6} {len(rows):3} to review  -> {target}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
