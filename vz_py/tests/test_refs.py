import html as html_lib
import itertools
import json
import re

import markdown
import pytest

from vhrd_zensical.code_refs import CodeRefError
from vhrd_zensical.glossary import GlossaryError

GLOSSARY = """
[[term]]
name = "piece"
forms = ["pieces"]
short = "One file of a tape, written in `blocks`."
chapter = "../pieces/"

[[term]]
name = "CRC"
long = "cyclic redundancy check"
short = "A checksum."
"""

BLOB = "https://github.com/o/r/blob/abc/"

RESUME_RS = "fn x() {}\n// --8<-- [start:call]\nlet w = writer.resume_on(cut)?;\n// --8<-- [end:call]\n"


_LINE = itertools.count(100, 10)


def item(path, kind="fn", doc="Does it.", lines=None, cfg=None, at=None):
    """An index entry; each at its own lines unless `at` gives another entry's (a re-export)"""
    if at is not None:
        return {**at, "path": path, "reexports": at["path"]}
    if lines is None:
        first = next(_LINE)
        lines = (first, first + 6)
    return {
        "path": path,
        "kind": kind,
        **({"cfg": cfg} if cfg else {}),
        "file": "src/lib.rs",
        "lines": list(lines),
        "source": f"pub {kind} {path.split('::')[-1]}()",
        "doc": doc,
        "url": f"{BLOB}src/lib.rs#L{lines[0]}-L{lines[1]}",
    }


LONG_DOC = "Reopen the piece.\n\nIt finds the cut and goes on: see [`Recovery`] and [`Gone`].\n\n" + "\n\n".join(
    f"Paragraph {i} " + "word " * 30 for i in range(10)
)


@pytest.fixture
def book(tmp_path, monkeypatch):
    monkeypatch.chdir(tmp_path)
    (tmp_path / "glossary.toml").write_text(GLOSSARY)
    (tmp_path / "tests").mkdir()
    (tmp_path / "tests" / "resume.rs").write_text(RESUME_RS)
    writer = item("tape::writer::TapeWriter", "struct")
    index = {
        "format": 2,
        "commit": "abc",
        "blob": BLOB,
        "items": [
            item("tape::writer::TapeWriter::resume_on", doc=LONG_DOC, lines=(3, 9)),
            writer,
            item("tape::Writer", at=writer),
            item("tape::TapeWriter", at=writer),
            item("tape::chain::resume"),
            item("tape::writer::resume"),
            item("tape_fs::writer::resume"),
            item("tape::hello", "macro"),
            item("tape::Recovery", "enum", lines=(20, 30)),
            item("tape::level", "struct"),
            item("tape::level", "fn"),
            item("tape::platform", cfg="unix"),
            item("tape::platform", cfg='not(feature = "std")'),
            item("tape::Evil", "struct", doc="Bad <script>alert(1)</script> and <img src=x onerror=alert(2)>.\n\n[click](javascript:alert(3))"),
        ],
        "files": [
            {"path": "tests/resume.rs", "lines": 4, "sections": {"call": [3, 3]}},
            {"path": "src/lib.rs", "lines": 40},
            {"path": "src/broken.rs", "lines": 3, "problem": "src/broken.rs:2: section `x` starts and never ends"},
        ],
    }
    (tmp_path / "index.json").write_text(json.dumps(index))
    return tmp_path


def render(book, text, abbr=False, **code):
    exts = ["vhrd_zensical.glossary", "vhrd_zensical.code_refs", *(["abbr"] if abbr else [])]
    return markdown.markdown(
        text,
        extensions=exts,
        extension_configs={
            "vhrd_zensical.glossary": {"file": "glossary.toml"},
            "vhrd_zensical.code_refs": {"index": "index.json", **code},
        },
    )


def cards(html):
    return re.findall(r'<div class="vz-card vz-card-\w+" id="([^"]+)"', html)


def test_a_reference_in_a_longer_code_span_stays_as_written(book):
    html = render(book, "Write ``[`resume_on`]`` or ``` [`resume.rs:call`] ``` and `` [[piece]] ``.")
    assert "<code>[`resume_on`]</code>" in html
    assert "<code>[`resume.rs:call`]</code>" in html
    assert "<code>[[piece]]</code>" in html
    assert not cards(html)
    assert "klzzwxh" not in html and "\x02" not in html


def test_a_reference_in_a_heading_is_plain_text(book):
    html = markdown.markdown(
        "## Resume a [[piece]] with [`resume_on`]\n\nA [[piece]] again.",
        extensions=["toc", "vhrd_zensical.glossary", "vhrd_zensical.code_refs"],
        extension_configs={
            "vhrd_zensical.glossary": {"file": "glossary.toml"},
            "vhrd_zensical.code_refs": {"index": "index.json"},
        },
    )
    assert '<h2 id="resume-a-piece-with-resume_on">Resume a piece with <code>resume_on</code></h2>' in html
    assert len(cards(html)) == 1


def test_a_term_is_a_plain_link_naming_its_card(book):
    html = render(book, "Each of the [[pieces]] and [[the file|piece]].")
    (cid,) = cards(html)
    assert f'<a aria-describedby="{cid}" class="vz-ref vz-term" data-vz-card="{cid}" href="/glossary/#term-piece">pieces</a>' in html
    assert ">the file</a>" in html
    assert "<strong>piece</strong>: One file of a tape, written in <code>blocks</code>." in html


def test_a_card_is_written_once_per_page(book):
    html = render(book, "[`resume_on`], [`TapeWriter::resume_on`] and [[piece]], [[pieces]].")
    assert len(cards(html)) == 2
    assert html.count('data-vz-card="') == 4
    assert html.index('class="vz-cards"') > html.index("pieces</a>")


def test_the_next_page_has_its_own_cards(book):
    md = markdown.Markdown(
        extensions=["vhrd_zensical.code_refs"], extension_configs={"vhrd_zensical.code_refs": {"index": "index.json"}}
    )
    assert len(cards(md.convert("[`resume_on`]"))) == 1
    md.reset()
    assert cards(md.convert("No references.")) == []


def test_an_unknown_term_fails(book):
    with pytest.raises(GlossaryError, match=r"\[\[chunk\]\]: no such term"):
        render(book, "A [[chunk]].")


def test_a_code_span_keeps_brackets_as_written(book):
    assert "<code>[[chunk]]</code>" in render(book, "Write `[[chunk]]` to mark a term.")
    assert "vz-ref" not in render(book, "Plain `resume.rs` and `Recovery`.")


def test_the_glossary_page_lists_every_entry(book):
    html = render(book, "# Glossary\n\n<!-- glossary -->\n")
    assert html.index('id="term-crc"') < html.index('id="term-piece"')
    assert "<strong>CRC (cyclic redundancy check)</strong>: A checksum." in html
    assert 'written in <code>blocks</code>. <a href="../pieces/">More</a>' in html


def test_abbreviations_get_their_long_form_with_abbr_on(book):
    assert '<abbr title="cyclic redundancy check">CRC</abbr>' in render(book, "Each CRC is checked.", abbr=True)
    assert "*[CRC]" not in render(book, "Each CRC is checked.")


@pytest.mark.parametrize("chapter", ["javascript:alert(1)", "https://x.org/", "//x.org/", "/abs/", 'a" onclick="x'])
def test_a_chapter_must_be_a_relative_page_link(book, chapter):
    (book / "glossary.toml").write_text(f'[[term]]\nname = "a"\nshort = "b"\nchapter = {json.dumps(chapter)}\n')
    with pytest.raises(GlossaryError, match="is no relative link"):
        render(book, "x")


def test_an_error_names_the_line(book):
    with pytest.raises(CodeRefError, match=r"^the page:4: \[`Gone`\]: no public item"):
        render(book, "First.\n\nSee [`hello!`] and\n[`Gone`].")
    with pytest.raises(GlossaryError, match=r"^the page:1: \[\[nope\]\]: no such term"):
        render(book, "A [[nope]].")


def test_an_error_names_the_page_zensical_renders(book):
    class Page:
        path = "crashes.md"

    class Context(markdown.preprocessors.Preprocessor):
        page = Page()
        config = {"docs_dir": "docs"}

        def run(self, lines):
            return lines

    class ContextExt(markdown.Extension):
        def extendMarkdown(self, md):
            md.preprocessors.register(Context(md), "rendering_context", 0)

    (book / "docs").mkdir()
    (book / "docs" / "crashes.md").write_text("---\ntitle: x\n---\n\n# Crashes\n\nSee [`Gone`].\n")
    with pytest.raises(CodeRefError, match=r"^docs/crashes.md:7: "):
        markdown.markdown(
            "# Crashes\n\nSee [`Gone`].\n",
            extensions=[ContextExt(), "vhrd_zensical.code_refs"],
            extension_configs={"vhrd_zensical.code_refs": {"index": "index.json"}},
        )


def test_a_word_naming_two_terms_fails(book):
    (book / "glossary.toml").write_text(GLOSSARY + '\n[[term]]\nname = "slice"\nforms = ["Piece"]\nshort = "x"\n')
    with pytest.raises(GlossaryError, match='"Piece" names two terms, "piece" and "slice"'):
        render(book, "x")


def test_an_item_card_renders_its_docs_and_highlights_its_signature(book):
    html = render(book, "Call [`TapeWriter::resume_on`] after a crash.")
    assert f'href="{BLOB}src/lib.rs#L3-L9"><code>TapeWriter::resume_on</code></a>' in html
    assert (
        f'<div class="vz-card-head"><a href="{BLOB}src/lib.rs#L3-L9">fn tape::writer::TapeWriter::resume_on</a></div>'
        in html
    )
    assert 'class="language-rust highlight"' in html
    # an intra-doc link that resolves opens the item's lines; one that does not stays plain code
    assert f'<a href="{BLOB}src/lib.rs#L20-L30"><code>Recovery</code></a>' in html
    assert "<code>Gone</code>" in html and ">Gone</a>" not in html
    assert '<div class="vz-card-where">src/lib.rs:3</div>' in html


def test_long_docs_show_the_summary_and_what_fits(book):
    html = render(book, "[`resume_on`]", card_doc_chars_max=350)
    assert "Reopen the piece." in html and "Paragraph 0" in html and "Paragraph 1" not in html
    assert "The docs go on in the source" in html
    assert "Paragraph 9" in render(book, "[`resume_on`]", card_doc_chars_max=100000)


def test_docs_are_text_never_markup(book):
    html = render(book, "[`Evil`]")
    assert "<script" not in html and "<img" not in html
    assert "&lt;script&gt;alert(1)&lt;/script&gt;" in html
    assert "onerror=alert(2)&gt;" in html
    assert "javascript:" not in html


def test_a_crate_prefix_narrows_the_match(book):
    assert "tape_fs::writer::resume" in render(book, "[`tape_fs::writer::resume`]")
    assert "<code>hello!</code>" in render(book, "[`hello!`]")


def test_an_ambiguous_name_says_how_to_write_each(book):
    with pytest.raises(CodeRefError, match=r"write one of \[`tape::chain::resume`\], \[`tape::writer::resume`\]"):
        render(book, "[`resume`]")
    with pytest.raises(CodeRefError, match=r"\[`struct@tape::level`\], \[`fn@tape::level`\]"):
        render(book, "[`level`]")
    with pytest.raises(CodeRefError, match=r'\[`cfg\(unix\)@tape::platform`\], \[`cfg\(not\(feature = "std"\)\)@tape::platform`\]'):
        render(book, "[`platform`]")


def test_a_kind_or_an_arm_picks_one_item(book):
    assert 'struct tape::level' in render(book, "[`type@level`]")
    assert 'fn tape::level' in render(book, "[`level()`]")
    html = render(book, '[`cfg(not(feature="std"))@platform`] and [`fn@cfg(unix)@platform`]')
    assert "<code>platform</code>" in html and "@" not in re.sub(r"<div class=\"vz-cards\">.*", "", html, flags=re.S)
    assert len(cards(html)) == 2
    assert html_lib.escape('fn tape::platform, cfg(not(feature = "std"))') in html
    with pytest.raises(CodeRefError, match="`thing@` is no kind"):
        render(book, "[`thing@level`]")


def test_a_reexport_is_the_same_item(book):
    html = render(book, "[`TapeWriter`], [`Writer`] and [`writer::TapeWriter`].")
    assert len(cards(html)) == 1
    assert "struct tape::TapeWriter" in html


def test_an_unknown_item_fails(book):
    with pytest.raises(CodeRefError, match="no public item"):
        render(book, "[`Gone`]")


def test_a_rustdoc_link_with_a_target_is_left_alone(book):
    assert "vz-ref" not in render(book, "[`Gone`](https://example.org)")


def test_a_section_card_shows_its_lines(book):
    html = render(book, "See [`resume.rs:call`].")
    assert f'href="{BLOB}tests/resume.rs#L3-L3"><code>resume.rs:call</code></a>' in html
    assert (
        f'<div class="vz-card-head"><a href="{BLOB}tests/resume.rs#L3-L3">tests/resume.rs, section call</a></div>'
        in html
    )
    assert "resume_on" in html and "fn x()" not in html
    assert '<div class="vz-card-where">tests/resume.rs:3-3</div>' in html
    links = re.findall(r'<div class="vz-card-links">(.*?)</div>', html)
    assert re.findall(r'<a href="([^"]+)">([^<]+)</a>', links[0]) == [
        (f"{BLOB}tests/resume.rs#L3-L3", "Lines on GitHub"),
        (f"{BLOB}tests/resume.rs", "File on GitHub"),
    ]


def test_a_file_card_is_bounded(book):
    html = render(book, "See [`tests/resume.rs`].", card_lines_max=2)
    assert f'href="{BLOB}tests/resume.rs"' in html
    assert "2 more lines in the source" in html
    links = re.findall(r'<div class="vz-card-links">(.*?)</div>', html)
    assert re.findall(r'<a href="([^"]+)">([^<]+)</a>', links[0]) == [(f"{BLOB}tests/resume.rs", "File on GitHub")]


def test_a_stale_file_or_section_fails(book):
    with pytest.raises(CodeRefError, match="no file git tracks"):
        render(book, "[`gone.rs`]")
    with pytest.raises(CodeRefError, match="marks no section `cut` \\(it marks: call\\)"):
        render(book, "[`resume.rs:cut`]")


def test_a_file_with_unpaired_markers_fails_only_where_referenced(book):
    assert "vz-card" in render(book, "[`resume.rs:call`]")
    with pytest.raises(CodeRefError, match=r"^the page:1: \[`broken.rs`\]: src/broken.rs:2: section `x` starts"):
        render(book, "[`broken.rs`]")


def test_a_missing_index_names_the_command(book):
    (book / "index.json").unlink()
    with pytest.raises(CodeRefError, match="run `vz prep`"):
        render(book, "x")


EVIL = "<script>alert(1)</script> <img src=x onerror=alert(1)>"


def assert_text(html):
    assert "<script" not in html and "<img" not in html, html
    assert "&lt;script&gt;alert(1)&lt;/script&gt;" in html and "&lt;img src=x onerror=alert(1)&gt;" in html, html


def test_a_doc_comment_is_text(book):
    index = json.loads((book / "index.json").read_text())
    index["items"].append(item("tape::Evil2", "struct", doc=EVIL))
    (book / "index.json").write_text(json.dumps(index))
    assert_text(render(book, "[`Evil2`]"))


def test_a_glossary_definition_and_name_are_text(book):
    (book / "glossary.toml").write_text(f'[[term]]\nname = "{EVIL}"\nforms = ["evil"]\nshort = "{EVIL}"\n')
    assert_text(render(book, "An [[evil]] term."))


def test_a_signature_with_generics_comes_out_whole(book):
    index = json.loads((book / "index.json").read_text())
    generic = item("tape::open", doc="Opens a `Sink<S>`.")
    generic["source"] = "pub fn open<S: Sink>(s: S) -> Result<(TapeWriter, Resumed), WriteError<S::Error>>"
    index["items"].append(generic)
    (book / "index.json").write_text(json.dumps(index))
    html = render(book, "[`open`]")
    # highlighted, the signature is split into spans; its text is whole and no part of it is a tag
    assert "<S:" not in html and "<S::" not in html
    text = html_lib.unescape(re.sub(r"<[^>]+>", "", html))
    assert "pub fn open<S: Sink>(s: S) -> Result<(TapeWriter, Resumed), WriteError<S::Error>>" in text
    assert "<code>Sink&lt;S&gt;</code>" in html


def _with_crates(book, crates):
    path = book / "index.json"
    data = json.loads(path.read_text())
    data["crates"] = crates
    data["items"] += [
        item("tape::Bits", "struct", lines=(60, 66)),
        item("tape::Bits::word", lines=(62, 64)),
        item("tape::Outcome", "enum", lines=(70, 75)),
        item("tape::Outcome::Done", "variant", lines=(72, 72)),
        item("tape::hidden::deep", "fn", lines=(80, 82)),
        item("tape::fmt", "macro", lines=(90, 92)),
    ]
    path.write_text(json.dumps(data))


def test_a_card_links_to_the_source_and_docs_rs_for_a_published_crate(book):
    _with_crates(book, {"tape": {"package": "tape-core", "published": True}})
    for text, docs in [
        ("[`tape::level()`]", "https://docs.rs/tape-core/latest/tape/fn.level.html"),
        ("[`tape::hidden::deep`]", "https://docs.rs/tape-core/latest/tape/hidden/fn.deep.html"),
        ("[`tape::Bits`]", "https://docs.rs/tape-core/latest/tape/struct.Bits.html"),
        ("[`tape::Bits::word`]", "https://docs.rs/tape-core/latest/tape/struct.Bits.html#method.word"),
        ("[`tape::Outcome::Done`]", "https://docs.rs/tape-core/latest/tape/enum.Outcome.html#variant.Done"),
        ("[`tape::fmt!`]", "https://docs.rs/tape-core/latest/tape/macro.fmt.html"),
    ]:
        html = render(book, text)
        links = re.findall(r'<div class="vz-card-links">(.*?)</div>', html)
        assert len(links) == 1, text
        hrefs = re.findall(r'<a href="([^"]+)">([^<]+)</a>', links[0])
        assert [t for _, t in hrefs] == ["Source on GitHub", "File on GitHub", "docs.rs"], text
        assert hrefs[0][0].startswith(f"{BLOB}src/lib.rs#L"), text
        assert hrefs[1][0] == f"{BLOB}src/lib.rs", text
        assert hrefs[2][0] == docs, text


def test_a_card_of_an_unpublished_crate_links_to_the_source_alone(book):
    _with_crates(book, {"tape": {"package": "tape-core", "published": False}})
    html = render(book, "[`tape::Bits`]")
    links = re.findall(r'<div class="vz-card-links">(.*?)</div>', html)
    hrefs = re.findall(r'<a href="([^"]+)">([^<]+)</a>', links[0])
    assert [t for _, t in hrefs] == ["Source on GitHub", "File on GitHub"]
    assert "docs.rs" not in html


def test_an_index_without_crates_links_to_the_source_alone(book):
    html = render(book, "[`tape::level()`]")
    assert "Source on GitHub" in html and "docs.rs" not in html
