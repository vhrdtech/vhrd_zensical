"""A book that copies a page from a second repository: `vz refs` indexes both workspaces from the sources file, and
the code link extension resolves the copied page's links against its source first.

The fixtures are `vz_refs/tests/projects/book` and `vz_refs/tests/projects/la`, copied side by side.
"""

import shutil
from pathlib import Path

import markdown
import pytest

from vhrd_zensical import _vz
from vhrd_zensical.code_refs import CodeRefError

PROJECTS = Path(__file__).resolve().parents[2] / "vz_refs" / "tests" / "projects"
LA = "https://github.com/o/la/blob/0123456789abcdef0123456789abcdef01234567/"
BOOK_COMMIT = "fedcba9876543210fedcba9876543210fedcba98"


@pytest.fixture
def book(tmp_path, monkeypatch):
    for name in ["book", "la"]:
        shutil.copytree(PROJECTS / name, tmp_path / name)
    monkeypatch.chdir(tmp_path / "book")
    assert _vz.main(["vz", "refs", "--commit", BOOK_COMMIT]) == 0
    return tmp_path / "book"


def render(book, page):
    class Page:
        path = page

    class Context(markdown.preprocessors.Preprocessor):
        page = Page()
        config = {"docs_dir": "docs"}

        def run(self, lines):
            return lines

    class ContextExt(markdown.Extension):
        def extendMarkdown(self, md):
            md.preprocessors.register(Context(md), "rendering_context", 0)

    return markdown.markdown(
        (book / "docs" / page).read_text(),
        extensions=[ContextExt(), "vhrd_zensical.code_refs"],
        extension_configs={"vhrd_zensical.code_refs": {"sources": "sources.toml"}},
    )


def test_a_copied_page_links_its_source(book):
    html = render(book, "analyzer/la.md")
    assert f'href="{LA}src/lib.rs#L4-L7"' in html
    assert f'href="{LA}src/lib.rs#L10-L10"' in html
    assert "https://docs.rs/la/latest/la/struct.RawChunk.html" in html
    # the book's own crates after the source's
    assert f'href="https://github.com/o/demo/blob/{BOOK_COMMIT}/src/lib.rs#L4-L4"' in html


def test_an_unknown_name_on_a_copied_page_names_both_places(book):
    page = book / "docs" / "analyzer" / "la.md"
    page.write_text(page.read_text() + "\nAnd [`Gone`].\n")
    with pytest.raises(CodeRefError, match=r"\[`Gone`\]: no public item of la's or the book's crates has this name"):
        render(book, "analyzer/la.md")


def test_a_page_of_the_book_does_not_see_the_source(book):
    page = book / "docs" / "index.md"
    page.write_text("See [`RawChunk`].\n")
    with pytest.raises(CodeRefError, match=r"\[`RawChunk`\]: no public item of the book's crates has this name"):
        render(book, "index.md")


def test_an_index_older_than_the_sources_file_fails(book):
    sources = book / "sources.toml"
    sources.write_text(sources.read_text().replace("0123456789abcdef", "1123456789abcdef", 1))
    with pytest.raises(CodeRefError, match=r"does not hold source \"la\" at the sources file's commit"):
        render(book, "analyzer/la.md")
