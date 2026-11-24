"""`[[term]]`: a term of the book's glossary in prose, linked to its entry and shown on hover.

The glossary file (setting `file`, default `docs/glossary.toml`) holds one `[[term]]` table per term: its `name`,
the `forms` prose also uses (plurals), a one-sentence `short` definition, the `chapter` that explains it, and for an
abbreviation its `long` form. A `chapter` is a relative link to a page of the book (letters, digits and `._~%#/-`,
not starting with `/`); anything else fails the build, so the glossary cannot link out or run script. `[[pieces]]` finds `piece` by its forms; `[[the tail|tail]]` shows other words for a
term. A term the file lacks, or a word naming two terms, raises `GlossaryError`, which fails the build.

The glossary page holds the line `<!-- glossary -->`, which becomes every entry in order of name, each with the
anchor `#term-NAME` that references link to (setting `page`, default `/glossary/`). With the `abbr` extension on,
every page also gets each abbreviation's long form, shown on hover wherever the abbreviation is written.

This file and `vz_core::glossary` parse the same format (`vz_core/tests/fixtures/glossary`); a format change
updates both parsers and the fixtures together.
"""

from __future__ import annotations

import functools
import html
import re
import tomllib
from pathlib import Path

from markdown import Extension
from markdown.inlinepatterns import InlineProcessor
from markdown.preprocessors import Preprocessor

from vhrd_zensical import card

PATTERN = r"\[\[([^\[\]|`\n]+)(?:\|([^\[\]|`\n]+))?\]\]"
MARKER = "<!-- glossary -->"
_KEYS = {"name", "forms", "short", "chapter", "long"}
_RELATIVE_PAGE = re.compile(r"[A-Za-z0-9._~%#/-]+")


class GlossaryError(card.RefError):
    pass


def slug(name: str) -> str:
    return re.sub(r"[^a-z0-9]+", "-", name.lower()).strip("-")


def _is_relative_page(chapter: str) -> bool:
    return not chapter.startswith("/") and bool(_RELATIVE_PAGE.fullmatch(chapter))


class Glossary:
    def __init__(self, path: Path, page: str):
        try:
            data = tomllib.loads(path.read_text(encoding="utf-8"))
        except (OSError, tomllib.TOMLDecodeError) as e:
            raise GlossaryError(f"{path}: {e}") from e
        self.page = page
        terms = data.get("term", [])
        for t in terms:
            unknown = set(t) - _KEYS
            if unknown or "name" not in t or "short" not in t:
                raise GlossaryError(f"{path}: term {t.get('name', '?')!r} needs name and short, knows only {sorted(_KEYS)}")
            if not isinstance(t["name"], str) or not isinstance(t["short"], str):
                raise GlossaryError(f"{path}: term {t['name']!r}: name and short must be strings")
            forms = t.get("forms", [])
            if not isinstance(forms, list) or not all(isinstance(f, str) for f in forms):
                raise GlossaryError(f'{path}: term "{t["name"]}": forms must be a list of strings')
            long = t.get("long")
            if long is not None and not isinstance(long, str):
                raise GlossaryError(f'{path}: term "{t["name"]}": long must be a string')
            chapter = t.get("chapter")
            if chapter is not None and (not isinstance(chapter, str) or not _is_relative_page(chapter)):
                raise GlossaryError(
                    f'{path}: term "{t["name"]}": chapter "{chapter}" is no relative link to a page of the book, '
                    'such as "../pieces/"'
                )
        self.terms: list[dict] = sorted(terms, key=lambda t: t["name"].lower())
        self.by_form: dict[str, dict] = {}
        for t in terms:
            for form in [t["name"], *t.get("forms", [])]:
                key = form.strip().lower()
                if not key:
                    raise GlossaryError(f'{path}: term "{t["name"]}": a name or form is blank')
                if key in self.by_form and self.by_form[key] is not t:
                    other = self.by_form[key]["name"]
                    raise GlossaryError(f'{path}: "{form}" names two terms, "{other}" and "{t["name"]}"')
                self.by_form[key] = t

    def find(self, form: str) -> dict:
        t = self.by_form.get(form.strip().lower())
        if t is None:
            raise GlossaryError(f"[[{form}]]: no such term in the glossary")
        return t

    def href(self, t: dict) -> str:
        return f"{self.page}#term-{slug(t['name'])}"


@functools.cache
def _load(path: str, mtime: float, page: str) -> Glossary:
    return Glossary(Path(path), page)


def load(path: Path, page: str) -> Glossary:
    """The glossary at `path`, read once per change of the file"""
    try:
        mtime = path.stat().st_mtime
    except OSError as e:
        raise GlossaryError(f"{path}: {e}") from e
    return _load(str(path), mtime, page)


class TermPattern(InlineProcessor):
    def __init__(self, glossary: Glossary, md):
        super().__init__(PATTERN, md)
        self.glossary = glossary

    @card.located
    def handleMatch(self, m, data):
        shown, name = m.group(1), m.group(2) or m.group(1)
        t = self.glossary.find(name)
        card_id = card.page_cards(self.md).add(f"term {t['name']}", lambda i: self.build(i, t))
        return card.reference(shown, self.glossary.href(t), "term", card_id), m.start(0), m.end(0)

    def build(self, card_id: str, t: dict) -> str:
        head = t["name"] + (f" ({t['long']})" if t.get("long") else "")
        body = f"<strong>{card.escape(head)}</strong>: {card.render_inline(t['short'])}"
        return card.frame(card_id, "term", body)


class GlossaryPage(Preprocessor):
    """The marker on the glossary page becomes the entries; with `abbr` on, every page gets the long forms."""

    def __init__(self, md, glossary: Glossary):
        super().__init__(md)
        self.glossary = glossary

    def run(self, lines: list[str]) -> list[str]:
        out: list[str] = []
        for line in lines:
            if line.strip() != MARKER:
                out.append(line)
                continue
            for t in self.glossary.terms:
                name = html.escape(t["name"])
                if t.get("long"):
                    name += f" ({html.escape(t['long'])})"
                more = f' <a href="{html.escape(t["chapter"])}">More</a>' if t.get("chapter") else ""
                out.append(
                    f'<p class="vz-gloss" id="term-{slug(t["name"])}"><strong>{name}</strong>: '
                    f"{card.render_inline(t['short'])}{more}</p>"
                )
                out.append("")
        if "abbr" in self.md.parser.blockprocessors:
            abbrs = [f"*[{t['name']}]: {t['long']}" for t in self.glossary.terms if t.get("long")]
            if abbrs:
                out += ["", *abbrs]
        return out


class GlossaryExtension(Extension):
    def __init__(self, **kwargs):
        self.config = {
            "file": ["docs/glossary.toml", "the glossary's entries, from the project root"],
            "page": ["/glossary/", "where the glossary page is served"],
        }
        super().__init__(**kwargs)

    def extendMarkdown(self, md):
        self.md = md
        glossary = load(Path(self.getConfig("file")), self.getConfig("page"))
        md.registerExtension(self)
        card.install(md)
        md.preprocessors.register(GlossaryPage(md, glossary), "vz_glossary_page", 25)
        # after the backtick pattern (190), so a code span keeps `[[x]]` as written; before the link pattern
        # (160), which would take the brackets apart
        md.inlinePatterns.register(TermPattern(glossary, md), "vz_glossary_term", 185)

    def reset(self):
        card.reset(self.md)


def makeExtension(**kwargs):
    return GlossaryExtension(**kwargs)
