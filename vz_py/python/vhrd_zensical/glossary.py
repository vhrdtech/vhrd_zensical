"""`[[term]]`: a term of the book's glossary in prose, linked to its entry and shown on hover.

The glossary file (setting `file`, default `docs/glossary.toml`) holds one `[[term]]` table per term: its `name`,
the `forms` prose also uses (plurals), a one-sentence `short` definition, the `chapter` that explains it, and for an
abbreviation its `long` form. A `chapter` is a relative link to a page of the book (letters, digits and `._~%#/-`,
not starting with `/`); anything else fails the build, so the glossary cannot link out or run script. `[[pieces]]`
finds `piece` by its forms; `[[the tail|tail]]` shows other words for a term. A term the file lacks, or a word
naming two terms, raises `GlossaryError`, which fails the build.

The glossary page holds the line `<!-- glossary -->`, which becomes every entry in order of name, each with the
anchor `#term-NAME` that references link to (setting `page`, default `/glossary/`). With the `abbr` extension on,
every page also gets each abbreviation's long form, shown on hover wherever the abbreviation is written.

The file is parsed by `vz_core::glossary`, and the links, cards and entries are written by `vz_refs::glossary`,
through `vhrd_zensical._vz`.
"""

from __future__ import annotations

from markdown import Extension
from markdown.inlinepatterns import InlineProcessor
from markdown.preprocessors import Preprocessor

from vhrd_zensical import _vz, card
from vhrd_zensical._vz import GlossaryError

__all__ = ["GlossaryError", "makeExtension"]

PATTERN = r"\[\[([^\[\]|`\n]+)(?:\|([^\[\]|`\n]+))?\]\]"

load = card.loader(_vz.Glossary)


class TermPattern(InlineProcessor):
    def __init__(self, glossary: _vz.Glossary, md):
        super().__init__(PATTERN, md)
        self.glossary = glossary

    @card.located
    def handleMatch(self, m, data):
        shown, name = m.group(1), m.group(2) or m.group(1)
        term = self.glossary.find(name)
        card_id = card.page_cards(self.md).add(term.key, lambda i: _vz.Glossary.card(i, term, card.render))
        return card.reference(shown, term.href, "term", card_id), m.start(0), m.end(0)


class GlossaryPage(Preprocessor):
    """The marker on the glossary page becomes the entries; with `abbr` on, every page gets the long forms."""

    def __init__(self, md, glossary: _vz.Glossary):
        super().__init__(md)
        self.glossary = glossary

    def run(self, lines: list[str]) -> list[str]:
        return self.glossary.expand(lines, card.render, "abbr" in self.md.parser.blockprocessors)


class GlossaryExtension(Extension):
    def __init__(self, **kwargs):
        self.config = {
            "file": [_vz.DEFAULT_GLOSSARY, "the glossary's entries, from the project root"],
            "page": [_vz.DEFAULT_GLOSSARY_PAGE, "where the glossary page is served"],
        }
        super().__init__(**kwargs)

    def extendMarkdown(self, md):
        self.md = md
        glossary = load(str(self.getConfig("file")), str(self.getConfig("page")))
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
