"""The review extension on a page rendered in a fake Zensical context: the badge under the title, the marks on the
sections' headings and blocks, and nothing when the review file is missing or of another format.
"""

import json
import shutil
from pathlib import Path

import markdown
import pytest

from vhrd_zensical.review import ReviewError

FIXTURES = Path(__file__).resolve().parents[2] / "vz_core" / "tests" / "fixtures" / "review"

PAGE = """# Tracks

Under the title.

## Items and tracks

An item.

- a list

### Tape format { #tape-format }

Changed since.

## Open questions

Nobody read this.
"""


@pytest.fixture
def book(tmp_path, monkeypatch):
    monkeypatch.chdir(tmp_path)
    shutil.copy(FIXTURES / "ok.json", tmp_path / "review.json")
    return tmp_path


def render(text, page="tracks.md"):
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
        text,
        extensions=["toc", "attr_list", ContextExt(), "vhrd_zensical.review"],
        extension_configs={"vhrd_zensical.review": {"file": "review.json"}},
    )


def write(book, pages):
    (book / "review.json").write_text(json.dumps({"format": 1, "pages": pages}))


def test_the_badge_sits_under_the_title(book):
    html = render(PAGE)
    assert (
        '<h1 id="tracks">Tracks</h1>\n'
        '<p class="vz-review-badge" title="Last proofread 26 Nov 2026">Proofread 81% · changed since</p>'
    ) in html


@pytest.mark.parametrize(
    ("lines", "read", "stale", "text"),
    [
        (40, 40, 0, "Proofread"),
        (100, 82, 0, "Proofread 82%"),
        (100, 82, 1, "Proofread 82% · changed since"),
        (100, 100, 4, "Proofread · changed since"),
    ],
)
def test_the_badge_text(book, lines, read, stale, text):
    write(book, {"tracks.md": {"lines": lines, "read": read, "stale": stale, "reviewed": "2026-03-02"}})
    html = render(PAGE)
    assert f'<p class="vz-review-badge" title="Last proofread 2 Mar 2026">{text}</p>' in html


def test_no_badge_when_nothing_is_read(book):
    write(book, {"tracks.md": {"lines": 10, "read": 0, "stale": 3, "sections": []}})
    html = render(PAGE)
    assert "vz-review" not in html


def test_marks_on_headings_and_blocks(book):
    html = render(PAGE)
    assert '<p data-vz-review="read">Under the title.</p>' in html
    assert '<h2 data-vz-review="partial" id="items-and-tracks">Items and tracks</h2>' in html
    assert '<p data-vz-review="partial">An item.</p>' in html
    assert '<ul data-vz-review="partial">' in html
    assert '<h3 data-vz-review="stale" id="tape-format">Tape format</h3>' in html
    assert '<p data-vz-review="stale">Changed since.</p>' in html
    # unread: no mark
    assert '<h2 id="open-questions">Open questions</h2>' in html
    assert "<p>Nobody read this.</p>" in html
    # the title and the badge carry none
    assert '<h1 id="tracks">' in html


def test_a_page_with_marks_ends_with_the_toggle(book):
    html = render(PAGE)
    assert html.rstrip().endswith(
        '<p class="vz-review-switch"><button aria-pressed="false" class="vz-review-toggle" '
        'title="Show or hide the review marks (r)" type="button">Review marks</button></p>'
    )


def test_a_page_without_marks_has_no_toggle(book):
    write(book, {"tracks.md": {"lines": 10, "read": 10, "stale": 0, "sections": []}})
    html = render(PAGE)
    assert "vz-review-badge" in html
    assert "vz-review-switch" not in html


def test_a_page_the_file_does_not_list_shows_nothing(book):
    assert "vz-review" not in render(PAGE, page="other.md")


def test_a_missing_file_shows_nothing(book):
    (book / "review.json").unlink()
    html = render(PAGE)
    assert "vz-review" not in html
    assert html == markdown.markdown(PAGE, extensions=["toc", "attr_list"])


def test_an_unknown_format_shows_nothing(book):
    shutil.copy(FIXTURES / "other-format.json", book / "review.json")
    assert "vz-review" not in render(PAGE)


def test_a_malformed_file_fails(book):
    shutil.copy(FIXTURES / "err-counts.json", book / "review.json")
    with pytest.raises(ReviewError, match=r"^review.json: page \"a.md\""):
        render(PAGE)


def test_outside_zensical_shows_nothing(book):
    html = markdown.markdown(
        PAGE,
        extensions=["toc", "vhrd_zensical.review"],
        extension_configs={"vhrd_zensical.review": {"file": "review.json"}},
    )
    assert "vz-review" not in html


def test_the_page_shows_no_names_or_keys(book):
    write(
        book,
        {
            "tracks.md": {
                "lines": 10,
                "read": 10,
                "stale": 0,
                "reviewed": "2026-11-26",
                "reviewer": "Somebody",
                "key": "ssh-ed25519 AAAA",
                "sections": [{"id": None, "line": 1, "state": "read", "by": "Somebody"}],
            }
        },
    )
    html = render(PAGE)
    assert "Somebody" not in html and "ssh-ed25519" not in html
