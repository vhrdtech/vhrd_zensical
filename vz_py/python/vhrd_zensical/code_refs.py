"""Code references: `` [`TapeWriter::resume_on`] `` names a public item, `` [`resume.rs`] `` or `` [`crashes.rs:resume`] `` a
file or a marked section of one. A plain code span stays plain.

The names come from the code index (setting `index`, default `target/vz/code_index.json`), which `vz refs` (or
`vz prep`) writes from the code before each build. An item name matches an item whose path ends with it; a name
that starts with a crate's name matches only in that crate. A trailing `()` asks for a function and `!` for a macro.
Two items with one path, a type and a function or two `#[cfg]` arms, are told apart as rustdoc does, by a kind
before the name (`` [`fn@level`] ``, `` [`type@level`] ``), and by the arm's predicate (`` [`cfg(unix)@platform`] ``,
`` [`fn@cfg(unix)@platform`] ``). A re-exported item is one item under all its paths. A file name
matches a file git tracks whose path ends with it, and a section is one marked `--8<-- [start:NAME]` ... `--8<-- [end:NAME]`, as
the snippet includes mark them. A name that matches nothing, or several things, raises `CodeRefError`, which fails the
build: a renamed or removed item, file or section cannot leave a stale reference behind.

The link opens the lines on the forge at the commit the index was written from. An item's card links to the same
lines and, for a crate the project lists in `on_crates_io` of its code_refs settings, to the item's page on docs.rs;
any other crate has the first link alone. The card shows its kind and
path, its signature (or whole source when short), and its docs rendered as Markdown, with rustdoc's intra-doc links
resolved through the same index: to the named item's lines, or plain code when they name nothing. The docs show whole
while they fit in `card_doc_chars_max` characters of Markdown; longer ones show their blocks while they fit, a first
paragraph too long cut after the last sentence that fits, its first sentence always. A file card shows
the code, at most `card_lines_max` lines. The setting `crates` is read by `vz refs` alone.
"""

from __future__ import annotations

import functools
import json
import re
import xml.etree.ElementTree as etree
from pathlib import Path

from markdown import Extension
from markdown.inlinepatterns import BacktickInlineProcessor, InlineProcessor

from vhrd_zensical import card

# [`Name`], [`a::b::Name`], [`f()`], [`m!`], [`fn@Name`], [`cfg(unix)@Name`]: rustdoc's shortcut form, not followed
# by a link target
ITEM = (
    r"\[`(?:([a-z]+)@)?(?:(cfg\([^`]*\))@)?((?:[A-Za-z_][A-Za-z0-9_]*::)*[A-Za-z_][A-Za-z0-9_]*)(\(\)|!)?`\]"
    r"(?![\[(])"
)
# a kind rustdoc writes before a name, and the kinds of the index it stands for
KINDS = {
    "fn": {"fn"}, "method": {"fn"}, "struct": {"struct"}, "enum": {"enum"}, "union": {"union"},
    "trait": {"trait"}, "type": {"struct", "enum", "union", "trait", "type"}, "tyalias": {"type"},
    "const": {"const"}, "constant": {"const"}, "static": {"static"}, "value": {"fn", "const", "static"},
    "macro": {"macro"}, "field": {"field"}, "variant": {"variant"},
}
# a code span opened by two backticks or more, which may hold a reference as written: ``[`Name`]``; the empty
# group stands for the escaped backslashes the backtick processor's own pattern catches in its first group
LONG_CODE = r"(?<![\\`])()`{2,}"
# [`file.rs`], [`dir/file.rs`], [`file.rs:section`]
FILE = r"\[`((?:[\w.-]+/)*[\w-][\w.-]*\.[A-Za-z0-9]+)(?::([^`\s]+))?`\](?![\[(])"
# kept in sync with vz_core::code_index::FORMAT
FORMAT = 2

_PATH = re.compile(r"^(?:[a-z]+@)?((?:[A-Za-z_][A-Za-z0-9_]*::)*[A-Za-z_][A-Za-z0-9_]*)(?:\(\)|!)?$")
_RUSTDOC_LANGS = {"", "rust", "no_run", "ignore", "should_panic", "compile_fail", "edition2018", "edition2021", "edition2024"}
_LANGS = {"rs": "rust", "py": "python", "sh": "bash", "js": "javascript", "ts": "typescript", "h": "c", "yml": "yaml",
          "md": "markdown", "x": "text", "ld": "text"}


class CodeRefError(card.RefError):
    pass


class Index:
    def __init__(self, path: Path):
        try:
            data = json.loads(path.read_text(encoding="utf-8"))
        except FileNotFoundError as e:
            raise CodeRefError(f"{path} is missing: run `vz prep` before the build") from e
        except (OSError, ValueError) as e:
            raise CodeRefError(f"{path}: {e}") from e
        if not isinstance(data, dict) or data.get("format") != FORMAT:
            raise CodeRefError(f"{path} is not a code index of format {FORMAT}: rebuild it with `vz prep`")
        self.items: list[dict] = data["items"]
        self.files: list[dict] = data["files"]
        self.blob: str = data["blob"]
        self.crate_info: dict[str, dict] = data.get("crates", {})
        self.crates = {i["path"].split("::")[0] for i in self.items}
        # each definition by the path a card shows: the shortest that keeps the item's own name
        defined = {_site(i): i["path"].split("::")[-1] for i in self.items if "reexports" not in i}
        self.sites: dict[tuple, dict] = {}
        for i in sorted(
            self.items,
            key=lambda i: (i["path"].split("::")[-1] != defined.get(_site(i)), i["path"].count("::"), i["path"]),
        ):
            self.sites.setdefault(_site(i), i)

    def matches(self, name: str, kinds: set[str] | None = None, cfg: str | None = None) -> list[dict]:
        """The items `name` names, of `kinds` and in the arm `cfg` when given: one per definition, by its shortest path"""
        want = name.split("::")
        if want[0] in self.crates and len(want) > 1:
            found = [
                i
                for i in self.items
                if i["path"].split("::")[0] == want[0] and i["path"].split("::")[-(len(want) - 1) :] == want[1:]
            ]
        else:
            found = [i for i in self.items if i["path"].split("::")[-len(want) :] == want]
        if kinds is not None:
            found = [i for i in found if i["kind"] in kinds]
        if cfg is not None:
            found = [i for i in found if _bare(f"cfg({i.get('cfg', '')})") == _bare(cfg)]
        # a re-export lists the item again under its own path: one item all the same
        first = {_site(i): self.sites[_site(i)] for i in found}
        return sorted(first.values(), key=lambda i: (i["path"], i["lines"]))

    def find(self, name: str, kind: str | None = None, cfg: str | None = None, suffix: str = "") -> dict:
        written = f"[`{kind + '@' if kind else ''}{cfg + '@' if cfg else ''}{name}{suffix}`]"
        if kind is not None and kind not in KINDS:
            raise CodeRefError(f"{written}: `{kind}@` is no kind; the kinds are {', '.join(sorted(KINDS))}")
        kinds = KINDS[kind] if kind else {"()": {"fn"}, "!": {"macro"}}.get(suffix)
        found = self.matches(name, kinds, cfg)
        if not found:
            raise CodeRefError(f"{written}: no public item of the book's crates has this name")
        if len(found) > 1:
            forms = ", ".join(f"[`{_form(i, found)}`]" for i in found)
            raise CodeRefError(f"{written}: several items have this name; write one of {forms}")
        return found[0]

    def near(self, name: str, at: dict) -> dict | None:
        """The item an intra-doc link in `at`'s docs names, or None: the one match, else the one in `at`'s crate"""
        found = self.matches(name)
        if len(found) > 1:
            crate = at["path"].split("::")[0]
            found = [i for i in found if i["path"].split("::")[0] == crate]
        return found[0] if len(found) == 1 else None

    def file(self, name: str) -> dict:
        want = name.split("/")
        found = [f for f in self.files if f["path"].split("/")[-len(want) :] == want]
        if not found:
            hint = "" if self.files else " (the index lists no files: vz refs ran outside a git checkout)"
            raise CodeRefError(f"[`{name}`]: no file git tracks has this name{hint}")
        if len(found) > 1:
            paths = ", ".join(f["path"] for f in found)
            raise CodeRefError(f"[`{name}`]: several files have this name ({paths}); name more of the path")
        return found[0]


_ANCHORS = {"fn": "method", "field": "structfield", "variant": "variant", "const": "associatedconstant", "type": "associatedtype"}
_MEMBER_OF = {"struct", "enum", "union", "trait", "type"}


def docs_rs_url(item: dict, index: Index) -> str | None:
    """The item's page on docs.rs, or None for a crate that is not published (or one the index knows nothing of)"""
    path = item["path"].split("::")
    info = index.crate_info.get(path[0])
    if not info or not info.get("published"):
        return None
    root = f"https://docs.rs/{info['package']}/latest/{path[0]}/"
    kind, name = item["kind"], path[-1]
    if kind == "macro":
        return f"{root}macro.{name}.html"
    owner = None
    if len(path) > 2 and kind in _ANCHORS:
        # a member sits on its type's page; its path may name the module of the `impl`, not the type's own
        types = [
            i
            for i in index.items
            if i["kind"] in _MEMBER_OF and i["path"].split("::")[0] == path[0] and "reexports" not in i
        ]
        owner = next((i for i in types if i["path"] == "::".join(path[:-1])), None) or next(
            (i for i in types if i["path"].split("::")[-1] == path[-2]), None
        )
        if owner is None and kind in ("field", "variant"):
            return None
    if owner is not None:
        mods = owner["path"].split("::")[1:-1]
        page = f"{owner['kind']}.{owner['path'].split('::')[-1]}.html"
        return f"{root}{''.join(m + '/' for m in mods)}{page}#{_ANCHORS[kind]}.{name}"
    mods = path[1:-1]
    return f"{root}{''.join(m + '/' for m in mods)}{kind}.{name}.html"


def _site(item: dict) -> tuple:
    return item["file"], tuple(item["lines"])


def _bare(cfg: str) -> str:
    return re.sub(r"\s+", "", cfg)


def _form(item: dict, among: list[dict]) -> str:
    """How a reference names `item` alone among `among`: its path, with its kind and arm where others share it"""
    same = [i for i in among if i["path"] == item["path"]]
    kind = f"{item['kind']}@" if len({i["kind"] for i in same}) > 1 else ""
    cfg = f"cfg({item['cfg']})@" if item.get("cfg") and len({i.get("cfg") for i in same}) > 1 else ""
    return f"{kind}{cfg}{item['path']}"


@functools.cache
def _load(path: str, mtime: float) -> Index:
    return Index(Path(path))


def load(path: Path) -> Index:
    """The index at `path`, read once per change of the file"""
    try:
        mtime = path.stat().st_mtime
    except FileNotFoundError as e:
        raise CodeRefError(f"{path} is missing: run `vz prep` before the build") from e
    return _load(str(path), mtime)


def _outside_fences(text: str, fix) -> str:
    """`fix` applied to the parts of a doc outside fenced code; rustdoc's hidden lines dropped inside rust fences"""
    out: list[str] = []
    prose: list[str] = []
    fence: str | None = None
    hide = False
    for line in text.split("\n"):
        stripped = line.strip()
        if fence is None and (m := re.match(r"^(`{3,}|~{3,})\s*([\w,-]*)", stripped)):
            out.append(fix("\n".join(prose)))
            prose = []
            fence = m.group(1)
            langs = set(m.group(2).split(",")) if m.group(2) else {""}
            hide = langs <= _RUSTDOC_LANGS
            out.append(fence + ("rust" if hide else m.group(2)))
        elif fence is not None and stripped.startswith(fence):
            out.append(fence)
            fence = None
        elif fence is not None:
            if not (hide and (stripped == "#" or stripped.startswith("# "))):
                out.append(line)
        else:
            prose.append(line)
    out.append(fix("\n".join(prose)))
    return "\n".join(out)


def doc_links(doc: str, at: dict, index: Index) -> str:
    """rustdoc's intra-doc links in `at`'s docs as Markdown links to the items' lines, or plain text"""
    defs: dict[str, str] = {}

    def strip_defs(text: str) -> str:
        lines = []
        for line in text.split("\n"):
            if m := re.match(r"^\s*\[([^\]]+)\]:\s*(\S+)\s*$", line):
                defs[m.group(1).strip("`")] = m.group(2)
            else:
                lines.append(line)
        return "\n".join(lines)

    parent = "::".join(at["path"].split("::")[:-1]) if at["kind"] in ("fn", "const", "field", "variant") else at["path"]

    def url(target: str) -> str | None:
        if target.startswith(("https://", "http://")):
            return target
        m = _PATH.match(target.strip("`"))
        if not m:
            return None
        name = m.group(1)
        if name.startswith("Self::"):
            name = parent + name[4:]
        name = re.sub(r"^(crate|self|super)::", "", name)
        item = index.near(name, at)
        return item["url"] if item else None

    def link(text: str, target: str) -> str:
        u = url(target)
        return f"[{text}]({u})" if u else text

    def fix(text: str) -> str:
        text = re.sub(r"\[([^\[\]]+)\]\(([^()\s]+)\)", lambda m: link(m.group(1), m.group(2)), text)
        text = re.sub(
            r"\[([^\[\]]+)\]\[([^\[\]]*)\]",
            lambda m: link(m.group(1), defs.get(m.group(2) or m.group(1).strip("`"), m.group(2) or m.group(1))),
            text,
        )
        return re.sub(
            r"\[([^\[\]]+)\](?![(\[:])",
            lambda m: link(m.group(1), defs.get(m.group(1).strip("`"), m.group(1)))
            if _PATH.match(m.group(1).strip("`")) or m.group(1).strip("`") in defs
            else m.group(0),
            text,
        )

    return _outside_fences(doc, lambda t: fix(strip_defs(t)))


def bounded(doc: str, chars_max: int) -> tuple[str, bool]:
    """The blocks of `doc` that fit in `chars_max`, a first paragraph cut at a sentence; whether any was left out"""
    blocks: list[str] = []
    current: list[str] = []
    fence: str | None = None
    for line in doc.split("\n"):
        stripped = line.strip()
        if fence is None and (m := re.match(r"^(`{3,}|~{3,})", stripped)):
            fence = m.group(1)
        elif fence is not None and stripped.startswith(fence):
            fence = None
        if fence is None and not stripped and current:
            blocks.append("\n".join(current))
            current = []
        elif stripped or fence is not None:
            current.append(line)
    if current:
        blocks.append("\n".join(current))
    kept: list[str] = []
    size = 0
    for b in blocks:
        if size + len(b) <= chars_max:
            kept.append(b)
            size += len(b)
            continue
        if not kept:
            kept.append(b if re.match(r"\s*(`{3,}|~{3,})", b) else _sentences(b, chars_max))
        break
    out = "\n\n".join(kept)
    return out, out != "\n\n".join(blocks)


def _sentences(paragraph: str, chars_max: int) -> str:
    """The sentences of `paragraph` that fit in `chars_max`, the first always; a code span is never cut"""
    ends = [
        m.end()
        for m in re.finditer(r"[.!?](?=\s+[A-Z`\[(*_])", paragraph)
        if paragraph.count("`", 0, m.start()) % 2 == 0
    ]
    fit = [e for e in ends if e <= chars_max]
    end = fit[-1] if fit else (ends[0] if ends else len(paragraph))
    return paragraph[:end]


def _lines(text: str, lines_max: int) -> tuple[str, int]:
    """At most `lines_max` lines of `text`, and how many were left out"""
    lines = text.split("\n")
    return "\n".join(lines[:lines_max]), max(0, len(lines) - lines_max)


def _more(n: int) -> str:
    return card.part("vz-card-more", f"{n} more line{'s' if n != 1 else ''} in the source") if n else ""


def _head(text: str, href: str) -> str:
    """A card's first line, a link to what it names on the forge"""
    return card.part("vz-card-head", f'<a href="{card.escape(href)}">{card.escape(text)}</a>')


def _links(links: list[tuple[str, str]]) -> str:
    """A card's links, text and address each"""
    return card.part(
        "vz-card-links", " · ".join(f'<a href="{card.escape(u)}">{card.escape(t)}</a>' for t, u in links)
    )


class _Refs(InlineProcessor):
    def __init__(self, pattern: str, ext: CodeRefExtension, index: Index):
        super().__init__(pattern, ext.md)
        self.ext = ext
        self.index = index


class ItemPattern(_Refs):
    @card.located
    def handleMatch(self, m, data):
        kind, cfg, name, suffix = m.group(1), m.group(2), m.group(3), m.group(4) or ""
        item = self.index.find(name, kind, cfg, suffix)
        key = f"item {item['path']} {item['file']}:{item['lines'][0]}"
        card_id = card.page_cards(self.md).add(key, lambda i: self.build(i, item))
        code_el = etree.Element("code")
        code_el.text = name + suffix
        return card.reference(code_el, item["url"], "code", card_id), m.start(0), m.end(0)

    def build(self, card_id: str, item: dict) -> str:
        src, left = _lines(item["source"], self.ext.lines_max)
        arm = f", cfg({item['cfg']})" if item.get("cfg") else ""
        body = _head(f"{item['kind']} {item['path']}{arm}", item["url"])
        body += card.part("vz-card-src", card.code(src, "rust")) + _more(left)
        if item["doc"]:
            doc, cut = bounded(doc_links(item["doc"], item, self.index), self.ext.doc_chars_max)
            body += card.part("vz-card-doc", card.render(doc))
            if cut:
                body += card.part("vz-card-more", "The docs go on in the source")
        body += card.part("vz-card-where", card.escape(f"{item['file']}:{item['lines'][0]}"))
        links = [("Source on GitHub", item["url"]), ("File on GitHub", item["url"].split("#", 1)[0])]
        if docs := docs_rs_url(item, self.index):
            links.append(("docs.rs", docs))
        body += _links(links)
        return card.frame(card_id, "code", body)


class FilePattern(_Refs):
    @card.located
    def handleMatch(self, m, data):
        name, section = m.group(1), m.group(2)
        f = self.index.file(name)
        if f.get("problem"):
            raise CodeRefError(f"[`{m.group(0)[2:-2]}`]: {f['problem']}")
        if section is None:
            first, last = 1, f["lines"]
            href = self.index.blob + f["path"]
        else:
            span = f.get("sections", {}).get(section)
            if span is None:
                known = ", ".join(sorted(f.get("sections", {}))) or "none"
                raise CodeRefError(f"[`{name}:{section}`]: {f['path']} marks no section `{section}` (it marks: {known})")
            first, last = span
            href = f"{self.index.blob}{f['path']}#L{first}-L{last}"
        key = f"file {f['path']}:{section or ''}"
        card_id = card.page_cards(self.md).add(key, lambda i: self.build(i, f, section, first, last, href))
        code_el = etree.Element("code")
        code_el.text = m.group(0)[2:-2]
        return card.reference(code_el, href, "file", card_id), m.start(0), m.end(0)

    def build(self, card_id: str, f: dict, section: str | None, first: int, last: int, href: str) -> str:
        try:
            lines = Path(f["path"]).read_text(encoding="utf-8").split("\n")[first - 1 : last]
        except OSError as e:
            raise CodeRefError(f"{f['path']}: {e}") from e
        src, left = _lines("\n".join(lines), self.ext.lines_max)
        ext = f["path"].rsplit(".", 1)[-1].lower()
        head = f["path"] + (f", section {section}" if section else "")
        body = _head(head, href)
        body += card.part("vz-card-src", card.code(src, _LANGS.get(ext, ext))) + _more(left)
        body += card.part("vz-card-where", card.escape(f"{f['path']}:{first}-{last}"))
        whole = self.index.blob + f["path"]
        links = [("Lines on GitHub", href)] if section is not None else []
        body += _links(links + [("File on GitHub", whole)])
        return card.frame(card_id, "file", body)


class CodeRefExtension(Extension):
    def __init__(self, **kwargs):
        self.config = {
            "index": ["target/vz/code_index.json", "the code index, from the project root"],
            "crates": [[], "the library crates vz refs indexes; empty: every library of the workspace"],
            "on_crates_io": [[], "the packages published on crates.io, read by vz refs alone"],
            "card_lines_max": [16, "lines of code a card shows at most"],
            "card_doc_chars_max": [400, "characters of docs a card shows at most; the first sentence always shows"],
        }
        super().__init__(**kwargs)

    def extendMarkdown(self, md):
        self.md = md
        self.lines_max = int(self.getConfig("card_lines_max"))
        self.doc_chars_max = int(self.getConfig("card_doc_chars_max"))
        index = load(Path(self.getConfig("index")))
        md.registerExtension(self)
        card.install(md)
        # before the backtick pattern (190), which would take the code span first; a longer code span before both,
        # so a reference written inside one stays as written
        md.inlinePatterns.register(BacktickInlineProcessor(LONG_CODE), "vz_long_code", 196)
        md.inlinePatterns.register(ItemPattern(ITEM, self, index), "vz_code_ref", 195)
        md.inlinePatterns.register(FilePattern(FILE, self, index), "vz_file_ref", 194)

    def reset(self):
        card.reset(self.md)


def makeExtension(**kwargs):
    return CodeRefExtension(**kwargs)
