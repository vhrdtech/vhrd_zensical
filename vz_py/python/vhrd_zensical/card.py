"""The cards references show, collected once per page, and the safe renderer that writes their contents.

A reference is a plain link: to the glossary entry, or to the lines on the forge. It names its card by id
(`data-vz-card`, `aria-describedby`), so without the script it still works as a link. Each distinct card is written
once per page, at the end of the page's content in a `vz-cards` block the stylesheet hides; the script shows a card
beside its reference on hover, on keyboard focus and on a first tap. A reference in a heading is plain text there, so
the heading's anchor and its line in the table of contents hold its words alone.

The cards are assembled by `vhrd_zensical._vz`; their Markdown is rendered here by `render`, a Markdown converter
with raw HTML off, no images, and links only to `http` and `https` addresses, highlighting code as the site does: a
doc comment holding `<script>` or an `onerror` attribute comes out as text.

A reference that fails names its page and line: the page from Zensical's rendering context, the line from the page's
file, or from the Markdown given when there is no file.
"""

from __future__ import annotations

import functools
import xml.etree.ElementTree as etree
from collections.abc import Callable
from pathlib import Path

import markdown
from markdown.preprocessors import Preprocessor
from markdown.treeprocessors import Treeprocessor

from vhrd_zensical import _vz
from vhrd_zensical._vz import RefError

__all__ = ["RefError", "install", "loader", "located", "page", "page_cards", "reference", "render", "reset"]

_ATTR = "_vz_cards"
_SOURCE = "_vz_source"
_HEADINGS = {"h1", "h2", "h3", "h4", "h5", "h6"}
_SAFE_SCHEMES = ("https://", "http://")


def located(handle):
    """`handle` (an inline processor's `handleMatch`) with its errors naming the page and line of the reference"""

    @functools.wraps(handle)
    def run(self, m, data):
        try:
            return handle(self, m, data)
        except RefError as e:
            raise type(e)(f"{where(self.md, m.group(0))}: {e}") from None

    return run


def _context(md: markdown.Markdown) -> tuple[str | None, str]:
    """The page's path and the docs folder, from Zensical's rendering context when it renders"""
    for p in md.preprocessors:
        path = getattr(getattr(p, "page", None), "path", None)
        if isinstance(path, str):
            config = getattr(p, "config", None)
            docs = config.get("docs_dir", "docs") if isinstance(config, dict) else "docs"
            return path, docs
    return None, "docs"


def page(md: markdown.Markdown) -> str | None:
    """The page's path relative to the docs folder, from Zensical's rendering context when it renders"""
    return _context(md)[0]


def where(md: markdown.Markdown, text: str) -> str:
    """`page:line` of the first line holding `text`"""
    page, docs = _context(md)
    return _vz.locate(page, docs, getattr(md, _SOURCE, []), text)


class _Source(Preprocessor):
    """Keeps the page's Markdown, to say the line of a failing reference"""

    def run(self, lines: list[str]) -> list[str]:
        setattr(self.md, _SOURCE, list(lines))
        return lines


class Cards:
    """The distinct cards of one page, in order of first reference: id to HTML"""

    def __init__(self) -> None:
        self.html: dict[str, str] = {}

    def add(self, key: str, build: Callable[[str], str]) -> str:
        """The id of the card for `key`, built by `build(id)` at its first reference on the page"""
        card_id = _vz.card_id(key)
        if card_id not in self.html:
            self.html[card_id] = build(card_id)
        return card_id


def _is_ref(el: etree.Element) -> bool:
    return el.tag == "a" and "vz-ref" in (el.get("class") or "").split()


def _append(parent: etree.Element, at: int, text: str) -> None:
    """Adds `text` after child `at - 1` of `parent`, or to its own text when `at` is 0"""
    if not text:
        return
    if at == 0:
        parent.text = (parent.text or "") + text
    else:
        prev = parent[at - 1]
        prev.tail = (prev.tail or "") + text


def _plain(parent: etree.Element) -> None:
    """The references under `parent` replaced by what they show"""
    i = 0
    while i < len(parent):
        child = parent[i]
        if not _is_ref(child):
            _plain(child)
            i += 1
            continue
        inner = list(child)
        parent.remove(child)
        _append(parent, i, child.text or "")
        for n, el in enumerate(inner):
            parent.insert(i + n, el)
        _append(parent, i + len(inner), child.tail or "")
        i += len(inner)


class _CardsBlock(Treeprocessor):
    """Makes references in headings plain text, then appends the cards still referenced after the content"""

    def run(self, root: etree.Element) -> None:
        cards = page_cards(self.md)
        if not cards.html:
            return
        for el in list(root.iter()):
            if el.tag in _HEADINGS:
                _plain(el)
        used = {el.get("data-vz-card") for el in root.iter() if _is_ref(el)}
        kept = [html for card_id, html in cards.html.items() if card_id in used]
        if kept:
            block = etree.SubElement(root, "div", {"class": "vz-cards"})
            block.text = self.md.htmlStash.store("".join(kept))


def install(md: markdown.Markdown) -> None:
    """Gives `md` the page's card registry and the block that writes it; both extensions call it"""
    if "vz_cards" not in md.treeprocessors:
        # after the inline patterns (20), which add the cards, and before prettify (10)
        md.treeprocessors.register(_CardsBlock(md), "vz_cards", 15)
        # first, before any preprocessor changes the lines
        md.preprocessors.register(_Source(md), "vz_source", 100)
    reset(md)


def reset(md: markdown.Markdown) -> None:
    """A new page: no cards yet"""
    setattr(md, _ATTR, Cards())


def page_cards(md: markdown.Markdown) -> Cards:
    cards = getattr(md, _ATTR, None)
    if cards is None:
        cards = Cards()
        setattr(md, _ATTR, cards)
    return cards


def reference(content: etree.Element | str, href: str, kind: str, card_id: str) -> etree.Element:
    """The link a reference becomes: to `href`, naming its card"""
    link = etree.Element(
        "a", {"class": f"vz-ref vz-{kind}", "href": href, "data-vz-card": card_id, "aria-describedby": card_id}
    )
    if isinstance(content, str):
        link.text = content
    else:
        link.append(content)
    return link


class _Safe(Treeprocessor):
    """Links only to http and https; images become their alternative text"""

    def run(self, root: etree.Element) -> None:
        for el in list(root.iter()):
            if el.tag == "a" and not el.get("href", "").startswith(_SAFE_SCHEMES):
                el.tag = "span"
                el.attrib.clear()
            elif el.tag == "img":
                alt = el.get("alt", "")
                el.tag = "span"
                el.attrib.clear()
                el.text = alt


@functools.cache
def _renderer() -> markdown.Markdown:
    md = markdown.Markdown(
        extensions=["pymdownx.superfences", "pymdownx.highlight"],
        extension_configs={"pymdownx.highlight": {"pygments_lang_class": True}},
    )
    md.preprocessors.deregister("html_block")
    md.inlinePatterns.deregister("html")
    md.treeprocessors.register(_Safe(md), "vz_safe", 1)
    return md


def render(text: str) -> str:
    """Markdown as safe HTML, code blocks highlighted as the site highlights them"""
    md = _renderer()
    md.reset()
    return md.convert(text)


def _mtime(path: str | None) -> float | None:
    if path is None:
        return None
    try:
        return Path(path).stat().st_mtime
    except OSError:
        return None


def loader(make: Callable[..., object], files: int = 1) -> Callable[..., object]:
    """`make(*args)` once per change of the files its first `files` arguments name (`None`: no file); a missing file
    goes to `make`, which names it"""

    @functools.cache
    def cached(mtimes: tuple[float | None, ...], *args):
        return make(*args)

    def load(*args):
        return cached(tuple(_mtime(p) for p in args[:files]), *args)

    return load
