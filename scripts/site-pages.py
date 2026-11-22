#!/usr/bin/env python3
"""The product page in the app's ten languages, from one template each:

    scripts/site-pages.py           # write site/
    scripts/site-pages.py --check   # fail if site/ differs from the sources

site-src/index.html and site-src/privacy.html hold the pages with {{key}}
where words go; site-src/strings/<language>.toml the words, as HTML.
English is written to site/, the others to site/<language>/, each page
naming its others (hreflang) and listing them in its footer, each
language by its own name. No script and nothing from elsewhere: the site
stays as private as the app. Every language must have every key.
"""
import html
import re
import sys
import tomllib
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
SRC = REPO / "site-src"
OUT = REPO / "site"
# The app's languages, in its order: the html `lang`, and the folder.
LANGUAGES = [
    ("en", ""),
    ("ro", "ro"),
    ("de", "de"),
    ("fr", "fr"),
    ("es", "es"),
    ("it", "it"),
    ("pt-BR", "pt-br"),
    ("nl", "nl"),
    ("pl", "pl"),
    ("ja", "ja"),
]
PAGES = ["index.html", "privacy.html"]
# Where the site is: `hreflang` alternates must be absolute URLs; they
# were relative, which search engines do not take (BUILD-003).
SITE = "https://animaengine.eu/"
# The markup a translation may hold. A string is HTML, so a translator's
# `<script>`, an `onclick=` or a `javascript:` link would have gone onto
# the published page as it was (BUILD-003): anything past these is
# refused, here and by `--check` in CI.
TAGS = {"a", "code", "em", "strong"}
FORBIDDEN = re.compile(r"javascript:|\son[a-z]+\s*=|<!--", re.I)
PLACEHOLDER = re.compile(r"\{\{([a-z0-9_]+)\}\}")


def check_markup(lang, key, value):
    """Refuse a translation holding markup past [`TAGS`]."""
    tags = {t.lower() for t in re.findall(r"</?\s*([a-zA-Z][a-zA-Z0-9]*)", value)}
    if tags - TAGS or FORBIDDEN.search(value):
        raise SystemExit(
            f"site-src/strings/{lang}.toml: `{key}` holds markup a translation may not "
            f"({sorted(tags - TAGS) or FORBIDDEN.search(value).group(0)!r}); allowed: {sorted(TAGS)}"
        )


def strings(lang):
    path = SRC / "strings" / f"{lang}.toml"
    with path.open("rb") as f:
        return tomllib.load(f)


def absolute(to_folder, page):
    """The page's own URL, for `hreflang`."""
    there = f"{to_folder}/" if to_folder else ""
    name = "" if page == "index.html" else page
    return SITE + there + name


def href(from_folder, to_folder, page):
    """A relative link from a page in `from_folder` to `page` in `to_folder`."""
    up = "../" if from_folder else ""
    there = f"{to_folder}/" if to_folder else ""
    name = "" if page == "index.html" else page
    return (up + there + name) or "./"


def render(template, words, lang, folder, page, all_words):
    values = dict(words)
    values["lang"] = lang
    values["root"] = "../" if folder else ""
    alternates = [
        f'<link rel="alternate" hreflang="{other}" href="{absolute(other_folder, page)}">'
        for other, other_folder in LANGUAGES
    ]
    alternates.append(f'<link rel="alternate" hreflang="x-default" href="{absolute("", page)}">')
    values["alternates"] = "\n".join(alternates)
    links = []
    for other, other_folder in LANGUAGES:
        name = all_words[other]["autonym"]
        if other == lang:
            links.append(f'<strong aria-current="page">{name}</strong>')
        else:
            links.append(f'<a href="{href(folder, other_folder, page)}" hreflang="{other}" lang="{other}">{name}</a>')
    values["languages"] = f'{words["language_label"]}: ' + " · ".join(links)
    return substitute(template, values, f"site-src/strings/{lang}.toml", page)


def substitute(template, values, source, page):
    """`template` with each `{{key}}` its value, in one pass — what a value
    holds is never looked at for keys, and the order is the template's —
    and escaped where it sits in an attribute (`alt="{{demo_alt}}"`): a
    `"` there ended the attribute (BUILD-003). Between tags a value is
    HTML, as the strings are written."""
    def value(m):
        key = m.group(1)
        if key not in values:
            raise SystemExit(f"{source}: no `{key}` (used in {page})")
        before = template[: m.start()]
        in_tag = before.rfind("<") > before.rfind(">")
        return html.escape(values[key], quote=True) if in_tag else values[key]
    return PLACEHOLDER.sub(value, template)


def self_check():
    """The escaping and the markup check, on what they are for: a check
    letting these through would be checking nothing."""
    out = substitute('<p title="{{t}}">{{b}}</p>', {"t": 'a"b<', "b": "<strong>x</strong> {{t}}"}, "self", "self")
    assert out == '<p title="a&quot;b&lt;"><strong>x</strong> {{t}}</p>', out
    for bad in ['<script>x</script>', '<a href="javascript:x">y</a>', '<a onclick="x">y</a>', '<img src=x>']:
        try:
            check_markup("self", "k", bad)
        except SystemExit:
            continue
        raise AssertionError(f"not refused: {bad}")
    check_markup("self", "k", 'A <code>.deb</code>, <a href="https://x/">y</a>')


def build():
    self_check()
    all_words = {lang: strings(lang) for lang, _ in LANGUAGES}
    for lang, words in all_words.items():
        for key, value in words.items():
            check_markup(lang, key, value)
    english = set(all_words["en"])
    for lang, words in all_words.items():
        missing = english - set(words)
        extra = set(words) - english
        if missing or extra:
            raise SystemExit(f"site-src/strings/{lang}.toml: missing {sorted(missing)}, unknown {sorted(extra)}")
    pages = {}
    for page in PAGES:
        template = (SRC / page).read_text(encoding="utf-8")
        for lang, folder in LANGUAGES:
            target = OUT / folder / page if folder else OUT / page
            pages[target] = render(template, all_words[lang], lang, folder, page, all_words)
    return pages


def main():
    check = "--check" in sys.argv[1:]
    pages = build()
    stale = []
    for target, text in pages.items():
        if check:
            if not target.exists() or target.read_text(encoding="utf-8") != text:
                stale.append(target.relative_to(REPO))
        else:
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text(text, encoding="utf-8")
    if check:
        if stale:
            print("site/ is not what site-src/ makes; run scripts/site-pages.py:")
            for path in stale:
                print(f"  {path}")
            sys.exit(1)
        print(f"site/: {len(pages)} pages up to date")
    else:
        print(f"wrote {len(pages)} pages")


if __name__ == "__main__":
    main()
