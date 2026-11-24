"""Abbreviations with their long form on hover, from the book's abbreviations file.

The abbreviations file (setting `file`, default `docs/abbreviations.toml`) holds one abbreviation per key, flat:
`CAN = "Controller Area Network"`. This extension adds every entry to each page as a definition of Python-Markdown's
`abbr` extension, which marks each use of the abbreviation in the page's text as `<abbr title="long form">`; code
spans and code blocks keep it as written. The theme's `content.tooltips` feature shows the title on hover.

The `abbr` extension must be on: without it the long forms would silently be missing, so the build fails. A missing
or malformed file fails the build too.

The file is parsed by `vz_core::abbreviations` and the definitions written by `vz_refs::abbreviations`, through
`vhrd_zensical._vz`.
"""

from __future__ import annotations

from markdown import Extension
from markdown.preprocessors import Preprocessor

from vhrd_zensical import _vz, card
from vhrd_zensical._vz import AbbreviationError

__all__ = ["AbbreviationError", "makeExtension"]

load = card.loader(_vz.Abbreviations)


class Definitions(Preprocessor):
    """Appends each long form to the page as an `abbr` definition"""

    def __init__(self, md, path: str):
        super().__init__(md)
        self.path = path

    def run(self, lines: list[str]) -> list[str]:
        if "abbr" not in self.md.parser.blockprocessors:
            raise AbbreviationError(
                "vhrd_zensical.abbreviations needs the abbr extension: add `abbr = {}` to markdown_extensions"
            )
        return load(self.path).define(lines)


class AbbreviationsExtension(Extension):
    def __init__(self, **kwargs):
        self.config = {
            "file": [_vz.DEFAULT_ABBREVIATIONS, "the abbreviations with their long forms, from the project root"],
        }
        super().__init__(**kwargs)

    def extendMarkdown(self, md):
        md.registerExtension(self)
        # after the glossary page (25): the file's long form comes last and wins over a glossary term's `long`
        md.preprocessors.register(Definitions(md, str(self.getConfig("file"))), "vz_abbreviations", 24)


def makeExtension(**kwargs):
    return AbbreviationsExtension(**kwargs)
