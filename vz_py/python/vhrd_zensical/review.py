"""A page's proofreading: a small badge under its title, and review marks beside its sections a reader turns on.

The review file (setting `file`, default `target/vz/review.json`) is the book's `glossator book export`, written
before the build. A missing file, or one of a format this kit does not know, shows nothing, so a build without
glossator passes unchanged; a malformed file fails the build.

- The badge, under the page's first `h1`: `Proofread` when every line is proofread, `Proofread 82%` below that,
  with `· changed since` when proofread lines changed since; its tooltip names the last day. None when no line is
  proofread.
- The marks: each top-level block takes its section's state in `data-vz-review` (`read`, `stale`, `partial`;
  none for unread). A section runs from its heading to the next heading of any level below the title; the blocks
  before the first such heading are the part under the title. Headings are matched by the ids the `toc` extension
  (or an explicit `{#id}`) gives them, so this runs after it.
- A page with any mark ends with a button that turns them on and off; the `r` key does too. The stylesheet hides
  the marks and the button until the script (`vz assets`, `javascripts/vz-review.js`) runs.

The page is named by Zensical's rendering context: rendered outside it, a page shows nothing. The file is parsed
by `vz_core::review`, the badge and marks by `vz_refs::review`, through `vhrd_zensical._vz`.
"""

from __future__ import annotations

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
    """Writes the page's badge, its blocks' marks and the button that shows them"""

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
        title = next((el for el in blocks if el.tag == "h1"), None)
        kinds = [
            ("title", None)
            if el is title
            else ("heading", el.get("id"))
            if el.tag in _HEADINGS
            else ("other", None)
            for el in blocks
        ]
        marked = False
        for el, mark in zip(blocks, review.marks(kinds), strict=True):
            if mark is not None:
                el.set("data-vz-review", mark)
                marked = True
        badge = review.badge
        if badge is not None:
            text, tooltip = badge
            el = etree.Element("p", {"class": "vz-review-badge"})
            if tooltip:
                el.set("title", tooltip)
            el.text = text
            root.insert(0 if title is None else list(root).index(title) + 1, el)
        if marked:
            switch = etree.SubElement(root, "p", {"class": "vz-review-switch"})
            button = etree.SubElement(
                switch,
                "button",
                {
                    "type": "button",
                    "class": "vz-review-toggle",
                    "aria-pressed": "false",
                    "title": "Show or hide the review marks (r)",
                },
            )
            button.text = "Review marks"


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
