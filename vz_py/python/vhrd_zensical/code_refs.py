"""Code references: `` [`TapeWriter::resume_on`] `` names a public item, `` [`resume.rs`] `` or `` [`crashes.rs:resume`] `` a
file or a marked section of one. A plain code span stays plain.

The names come from the code index (setting `index`, default `target/vz/code_index.json`), which `vz refs` (or
`vz prep`) writes from the code before each build. An item name matches an item whose path ends with it; a name
that starts with a crate's name matches only in that crate, and an item's whole path matches that item alone. A
trailing `()` asks for a function and `!` for a macro. Two items with one path, a type and a function or two
`#[cfg]` arms, are told apart as rustdoc does, by a kind before the name (`` [`fn@level`] ``, `` [`type@level`] ``),
and by the arm's predicate (`` [`cfg(unix)@platform`] ``, `` [`fn@cfg(unix)@platform`] ``). A re-exported item is
one item under all its paths. A file name matches a file git tracks whose path ends with it, and a section is one
marked `--8<-- [start:NAME]` ... `--8<-- [end:NAME]`, as the snippet includes mark them. A name that matches
nothing, or several things, raises `CodeRefError`, which fails the build: a renamed or removed item, file or section
cannot leave a stale reference behind.

The link opens the lines on the forge at the commit the index was written from. An item's card links to the same
lines and, for a crate the project lists in `on_crates_io` of its code_refs settings, to the item's page on docs.rs;
any other crate has the first link alone. The card shows its kind and path, its signature (or whole source when
short), and its docs rendered as Markdown, with rustdoc's intra-doc links resolved through the same index: to the
named item's lines, or plain code when they name nothing. The docs show whole while they fit in
`card_doc_chars_max` characters of Markdown; longer ones show their blocks while they fit, a first paragraph too
long cut after the last sentence that fits, its first sentence always. A file card shows the code, at most
`card_lines_max` lines. The setting `crates` is read by `vz refs` alone.

The names are resolved and the cards assembled by `vz_refs::links`, through `vhrd_zensical._vz`; this module
registers the patterns and renders the cards' Markdown.
"""

from __future__ import annotations

import xml.etree.ElementTree as etree

from markdown import Extension
from markdown.inlinepatterns import InlineProcessor
from markdown.util import AtomicString, code_escape

from vhrd_zensical import _vz, card
from vhrd_zensical._vz import CodeRefError

__all__ = ["CodeRefError", "makeExtension"]

# [`Name`], [`a::b::Name`], [`f()`], [`m!`], [`fn@Name`], [`cfg(unix)@Name`]: rustdoc's shortcut form, not followed
# by a link target
ITEM = (
    r"\[`(?:([a-z]+)@)?(?:(cfg\([^`]*\))@)?((?:[A-Za-z_][A-Za-z0-9_]*::)*[A-Za-z_][A-Za-z0-9_]*)(\(\)|!)?`\]"
    r"(?![\[(])"
)
# a code span opened by two backticks or more, which may hold a reference as written: ``[`Name`]``
LONG_CODE = r"(?<![\\`])`{2,}"
# [`file.rs`], [`dir/file.rs`], [`file.rs:section`]
FILE = r"\[`((?:[\w.-]+/)*[\w-][\w.-]*\.[A-Za-z0-9]+)(?::([^`\s]+))?`\](?![\[(])"

load = card.loader(_vz.CodeIndex)


def _code(text: str) -> etree.Element:
    el = etree.Element("code")
    el.text = text
    return el


class LongCodeInlineProcessor(InlineProcessor):
    """A code span opened by two or more backticks, closed by a run of the same length, as CommonMark reads one.

    Written on its own rather than reused from `markdown.inlinepatterns.BacktickInlineProcessor`: that class reads
    its match through group indices fixed by its own pattern (`m.group(3)` on Markdown 3.7-3.10), which `LONG_CODE`
    does not have and which raises `IndexError` there; Markdown 3.11 changed that class to use `find_code_spans`
    instead, read through no group at all, but still its own pattern's start.
    """

    tag = "code"

    def handleMatch(self, m, data):
        begin = m.start(0)
        open_end = m.end(0)
        open_len = open_end - begin
        last = len(data)
        i = open_end
        while i < last:
            run_start = i
            while i < last and data[i] == "`":
                i += 1
            run_len = i - run_start
            if run_len == 0:
                i += 1
                continue
            if run_len == open_len:
                content = data[open_end:run_start]
                if len(content) >= 2 and content[0] == " " and content[-1] == " " and content.strip(" "):
                    content = content[1:-1]
                el = etree.Element(self.tag)
                el.text = AtomicString(code_escape(content))
                return el, begin, i
        # no closing run of the same length: leave the opening ticks as written
        return None, begin, open_end


class _Refs(InlineProcessor):
    def __init__(self, pattern: str, ext: CodeRefExtension, index: _vz.CodeIndex):
        super().__init__(pattern, ext.md)
        self.ext = ext
        self.index = index


class ItemPattern(_Refs):
    @card.located
    def handleMatch(self, m, data):
        item = self.index.item(m.group(3), m.group(1), m.group(2), m.group(4) or "")
        ext = self.ext
        card_id = card.page_cards(self.md).add(
            item.key, lambda i: self.index.item_card(i, item, card.render, ext.lines_max, ext.doc_chars_max)
        )
        return card.reference(_code(item.text), item.href, "code", card_id), m.start(0), m.end(0)


class FilePattern(_Refs):
    @card.located
    def handleMatch(self, m, data):
        f = self.index.file(m.group(1), m.group(2))
        lines_max = self.ext.lines_max
        card_id = card.page_cards(self.md).add(
            f.key, lambda i: self.index.file_card(i, f, card.render, lines_max)
        )
        return card.reference(_code(f.text), f.href, "file", card_id), m.start(0), m.end(0)


class CodeRefExtension(Extension):
    def __init__(self, **kwargs):
        self.config = {
            "index": [_vz.DEFAULT_CODE_INDEX, "the code index, from the project root"],
            "crates": [[], "the library crates vz refs indexes; empty: every library of the workspace"],
            "on_crates_io": [[], "the packages published on crates.io, read by vz refs alone"],
            "card_lines_max": [_vz.DEFAULT_CARD_LINES_MAX, "lines of code a card shows at most"],
            "card_doc_chars_max": [
                _vz.DEFAULT_CARD_DOC_CHARS_MAX,
                "characters of docs a card shows at most; the first sentence always shows",
            ],
        }
        super().__init__(**kwargs)

    def extendMarkdown(self, md):
        self.md = md
        self.lines_max = int(self.getConfig("card_lines_max"))
        self.doc_chars_max = int(self.getConfig("card_doc_chars_max"))
        index = load(str(self.getConfig("index")))
        md.registerExtension(self)
        card.install(md)
        # before the backtick pattern (190), which would take the code span first; a longer code span before both,
        # so a reference written inside one stays as written
        md.inlinePatterns.register(LongCodeInlineProcessor(LONG_CODE, md), "vz_long_code", 196)
        md.inlinePatterns.register(ItemPattern(ITEM, self, index), "vz_code_ref", 195)
        md.inlinePatterns.register(FilePattern(FILE, self, index), "vz_file_ref", 194)

    def reset(self):
        card.reset(self.md)


def makeExtension(**kwargs):
    return CodeRefExtension(**kwargs)
