"""A page's proofreading: a small badge under its title, and review marks beside its sections a reader turns on.

The review file (setting `file`, default `target/vz/review.json`) is the book's `glossator book export`, written
before the build. A missing file, or one of a format this kit does not know, shows nothing, so a build without
glossator passes unchanged; a malformed file fails the build.

- The badge, first on the page, plain text with the percent coloured: green only at 100%, amber when proofread
  lines changed since. The script puts a copy above the page's table of contents ("On this page"); where that
  sidebar is hidden, the badge stays at the right end of the first line: `Read` when every line is proofread, `Read 82%` below that,
  with `· changed since` when proofread lines changed since; its tooltip names the last day. None when no line is
  proofread.
- The marks: each top-level block takes the state of its source lines: `read` when every line is read, `stale` when
  one changed since, `partial` when some are read, none when none is. Each run of neighbouring blocks with the same
  mark goes into a `div.vz-review-run` with that state in `data-vz-review`; its bar, `span.vz-review-bar`, says on
  hover when they were read: `Read and signed 26 Nov 2026`. The blocks are found in the page's Markdown file (a
  snippet line counts the blocks of the file it includes); when they do not number as many as the page rendered, or
  the review file is from before line runs, each block takes its section's state instead: a section runs from its
  heading to the next heading below the title, matched by the ids `toc` gives (so this runs after it).
- On a page with any mark the badge is a button that turns them on and off; the `r` key does too. The marks stay
  hidden until the script (`vz assets`, `javascripts/vz-review.js`) turns them on.

The page is named by Zensical's rendering context: rendered outside it, a page shows nothing. The file is parsed
by `vz_core::review`, the badge and marks by `vz_refs::review`, through `vhrd_zensical._vz`.
"""

from __future__ import annotations

import pathlib
import re
import xml.etree.ElementTree as etree

from markdown import Extension
from markdown.treeprocessors import Treeprocessor

from vhrd_zensical import _vz, card
from vhrd_zensical._vz import ReviewError

__all__ = ["ReviewError", "makeExtension"]

load = card.loader(_vz.Review)

_HEADINGS = {"h1", "h2", "h3", "h4", "h5", "h6"}
# blocks the kit and Python-Markdown append after the content: no section's text
_APPENDED = {"vz-cards", "footnote"}


def _appended(el: etree.Element) -> bool:
    return bool(_APPENDED.intersection((el.get("class") or "").split()))


class Proofread(Treeprocessor):
    """Writes the page's badge and its sections' marks"""

    def __init__(self, md, path: str):
        super().__init__(md)
        self.path = path

    def run(self, root: etree.Element) -> None:
        page = card.page(self.md)
        if page is None:
            return
        review = load(self.path).page(page)
        if review is None:
            return
        blocks = [el for el in root if not _appended(el)]
        source = _source(self.md)
        marks = None if source is None else review.block_marks(source, len(blocks), _included(source))
        if marks is None:
            marks = _section_marks(review, blocks)
        marked = any(m is not None for m in marks)
        _wrap_runs(root, blocks, marks)
        if review.badge is not None:
            root.insert(0, _badge(*review.badge, toggle=marked))


_SNIPPET = re.compile(r"""^--8<--\s+["']([^"']+)["']\s*$""", re.M)
# bytes of a page or an included file read at most
_SOURCE_BYTES_MAX = 16 << 20


def _read(path: pathlib.Path) -> str | None:
    try:
        with path.open(encoding="utf-8") as f:
            return f.read(_SOURCE_BYTES_MAX)
    except (OSError, UnicodeDecodeError):
        return None


def _source(md) -> str | None:
    """The page's Markdown file, from the docs folder"""
    page, docs = card._context(md)
    return None if page is None else _read(pathlib.Path(docs) / page)


def _included(source: str) -> dict[str, int]:
    """The blocks of each file the page's snippet lines include, found from the project root as `snippets` does"""
    out = {}
    for path in set(_SNIPPET.findall(source)):
        text = _read(pathlib.Path(path))
        if text is not None:
            out[path] = len(_vz.md_blocks(text))
    return out


def _section_marks(review, blocks: list[etree.Element]) -> list[tuple[str, str] | None]:
    """Each block's mark from its section's state"""
    title = next((el for el in blocks if el.tag == "h1"), None)
    kinds = [
        ("title", None)
        if el is title
        else ("heading", el.get("id"))
        if el.tag in _HEADINGS
        else ("other", None)
        for el in blocks
    ]
    marks = list(review.marks(kinds))
    # blocks above the title (a hero picture) take no mark: the marks start at the title's line
    if title is not None:
        first = blocks.index(title)
        marks[:first] = [None] * first
    return marks


def _badge(_text: str, tooltip: str, percent: int, stale: bool, *, toggle: bool) -> etree.Element:
    """`Read 82%` with the percent coloured by its tone (full, part, stale) and `changed since` when stale.
    It goes first on the page; the script puts a copy above the page's table of contents. On a page with marks it is
    a button that turns them on and off."""
    tone = "stale" if stale else "full" if percent == 100 else "part"
    classes = "vz-review-badge vz-sidebar-badge"
    if toggle:
        tooltip = f"{tooltip}. Click or press r for the review marks" if tooltip else "Click or press r for the review marks"
        el = etree.Element("button", {"type": "button", "class": f"{classes} vz-review-toggle", "aria-pressed": "false"})
    else:
        el = etree.Element("span", {"class": classes})
    el.set("data-vz-review-tone", tone)
    if tooltip:
        el.set("title", tooltip)
    el.text, el.tail = "Read ", "\n"
    value = etree.SubElement(el, "span", {"class": "vz-review-percent"})
    value.text = f"{percent}%"
    if stale:
        value.tail = " \u00b7 "
        changed = etree.SubElement(el, "span", {"class": "vz-review-changed"})
        changed.text = "changed since"
    return el


def _wrap_runs(root: etree.Element, blocks: list[etree.Element], marks: list[tuple[str, str] | None]) -> None:
    """Puts each run of neighbouring blocks with the same mark into one `div`, so its bar runs along the text
    column's edge, also beside a narrower centred figure. The bar is the run's first child, its tooltip saying when
    the section was read."""
    run: list[etree.Element] = []
    mark: tuple[str, str] | None = None
    for el, m in [*zip(blocks, marks, strict=True), (None, None)]:
        if run and (m != mark or el is None or list(root).index(el) != list(root).index(run[-1]) + 1):
            assert mark is not None, "a run holds marked blocks only"
            state, tooltip = mark
            div = etree.Element("div", {"class": "vz-review-run", "data-vz-review": state})
            div.text, div.tail = "\n", "\n"
            bar = etree.SubElement(div, "span", {"class": "vz-review-bar", "title": tooltip})
            bar.tail = "\n"
            root.insert(list(root).index(run[0]), div)
            for b in run:
                root.remove(b)
                div.append(b)
            run = []
        if el is not None and m is not None:
            run.append(el)
            mark = m


class ReviewExtension(Extension):
    def __init__(self, **kwargs):
        self.config = {
            "file": [_vz.DEFAULT_REVIEW, "the review file (glossator book export), from the project root"],
        }
        super().__init__(**kwargs)

    def extendMarkdown(self, md):
        md.registerExtension(self)
        # after toc (5), which gives the headings their ids
        md.treeprocessors.register(Proofread(md, str(self.getConfig("file"))), "vz_review", 4)


def makeExtension(**kwargs):
    return ReviewExtension(**kwargs)
