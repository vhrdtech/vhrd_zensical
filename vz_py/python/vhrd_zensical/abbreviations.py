"""Abbreviations with their long form on hover, from the book's abbreviations file.

The abbreviations file (setting `file`, default `docs/abbreviations.toml`) holds one abbreviation per key, flat:
`CAN = "Controller Area Network"`. This extension adds every entry to each page as a definition of Python-Markdown's
`abbr` extension, which marks each use of the abbreviation in the page's text as `<abbr title="long form">`; code
spans and code blocks keep it as written. The theme's `content.tooltips` feature shows the title on hover.

The `abbr` extension must be on: without it the long forms would silently be missing, so the build fails. A missing
or malformed file fails the build too.

This file and `vz_core::abbreviations` parse the same format (`vz_core/tests/fixtures/abbreviations`); a format
change updates both parsers and the fixtures together.
"""

from __future__ import annotations

import functools
import tomllib
from pathlib import Path

from markdown import Extension
from markdown.preprocessors import Preprocessor

from vhrd_zensical import card


class AbbreviationError(card.RefError):
    pass


def _parse(text: str) -> tuple[tuple[str, str], ...]:
    try:
        data = tomllib.loads(text)
    except tomllib.TOMLDecodeError as e:
        raise AbbreviationError(str(e)) from e
    out = []
    for abbr, long in data.items():
        if not isinstance(long, str):
            raise AbbreviationError(f"{abbr!r}: a long form must be a string")
        if not abbr or abbr.strip() != abbr or any(c in abbr for c in "[]\n\r"):
            raise AbbreviationError(f"{abbr!r} is no abbreviation: one line, no surrounding space, no `[` or `]`")
        if not long.strip() or any(c in long for c in "\n\r"):
            raise AbbreviationError(f'"{abbr}": long form {long!r} is blank or spans lines')
        out.append((abbr, long))
    return tuple(sorted(out))


@functools.cache
def _load(path: str, mtime: float) -> tuple[tuple[str, str], ...]:
    return _parse(Path(path).read_text(encoding="utf-8"))


def load(path: Path) -> tuple[tuple[str, str], ...]:
    """The abbreviations at `path` as (abbreviation, long form) pairs, read once per change of the file"""
    try:
        mtime = path.stat().st_mtime
    except OSError as e:
        raise AbbreviationError(f"{path}: {e}") from e
    return _load(str(path), mtime)


class Definitions(Preprocessor):
    """Appends each long form to the page as an `abbr` definition"""

    def __init__(self, md, path: Path):
        super().__init__(md)
        self.path = path

    def run(self, lines: list[str]) -> list[str]:
        if "abbr" not in self.md.parser.blockprocessors:
            raise AbbreviationError(
                "vhrd_zensical.abbreviations needs the abbr extension: add `abbr = {}` to markdown_extensions"
            )
        table = load(self.path)
        if not table:
            return lines
        return [*lines, "", *(f"*[{abbr}]: {long}" for abbr, long in table)]


class AbbreviationsExtension(Extension):
    def __init__(self, **kwargs):
        self.config = {
            "file": ["docs/abbreviations.toml", "the abbreviations with their long forms, from the project root"],
        }
        super().__init__(**kwargs)

    def extendMarkdown(self, md):
        md.registerExtension(self)
        # after the glossary page (25): the file's long form comes last and wins over a glossary term's `long`
        md.preprocessors.register(Definitions(md, Path(self.getConfig("file"))), "vz_abbreviations", 24)


def makeExtension(**kwargs):
    return AbbreviationsExtension(**kwargs)
