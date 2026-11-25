"""The extension module: the ``vz`` command and the Markdown extensions' logic, built from the Rust workspace."""

from collections.abc import Callable

Render = Callable[[str], str]

DEFAULT_GLOSSARY: str
DEFAULT_GLOSSARY_PAGE: str
DEFAULT_CODE_INDEX: str
DEFAULT_ABBREVIATIONS: str
DEFAULT_CARD_LINES_MAX: int
DEFAULT_CARD_DOC_CHARS_MAX: int

class RefError(Exception):
    """A reference that cannot be resolved, or a book file that does not read; it fails the build"""

class GlossaryError(RefError): ...
class CodeRefError(RefError): ...
class AbbreviationError(RefError): ...

def main(argv: list[str] | None = None) -> int:
    """Run ``vz`` with ``argv`` (program name first; ``sys.argv`` when omitted) and return its exit status."""

def card_id(key: str) -> str:
    """The id of the card for ``key``"""

def locate(page: str | None, docs: str, source: list[str], text: str) -> str:
    """``page:line`` of the first line holding ``text``: the page's file read from ``docs``, else ``source``"""

class Term:
    key: str
    href: str

class Glossary:
    def __init__(self, file: str, page: str) -> None: ...
    def find(self, form: str) -> Term: ...
    @staticmethod
    def card(card_id: str, term: Term, render: Render) -> str: ...
    def expand(self, lines: list[str], render: Render, abbr: bool) -> list[str]: ...

class Item:
    key: str
    href: str
    text: str

class File:
    key: str
    href: str
    text: str

class CodeIndex:
    def __init__(self, file: str, sources: str | None = None) -> None: ...
    def item(
        self, name: str, kind: str | None = None, cfg: str | None = None, suffix: str = "", page: str | None = None
    ) -> Item: ...
    def file(self, name: str, section: str | None = None, page: str | None = None) -> File: ...
    def item_card(self, card_id: str, item: Item, render: Render, lines_max: int, doc_chars_max: int) -> str: ...
    def file_card(self, card_id: str, file: File, render: Render, lines_max: int) -> str: ...

class Abbreviations:
    def __init__(self, file: str) -> None: ...
    def define(self, lines: list[str]) -> list[str]: ...
