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


def test_the_badge_comes_first_and_toggles_the_marks(book):
    html = render(PAGE)
    assert html.startswith(
        '<button aria-pressed="false" class="vz-review-badge vz-sidebar-badge vz-review-toggle" '
        'data-vz-review-tone="stale" '
        'title="Last read and signed 26 Nov 2026. Click or press r for the review marks" type="button">'
        'Read <span class="vz-review-percent">81%</span> \u00b7 <span class="vz-review-changed">changed since</span>'
        "</button>\n"
    )


@pytest.mark.parametrize(
    ("lines", "read", "stale", "tone", "inner"),
    [
        (40, 40, 0, "full", '<span class="vz-review-percent">100%</span>'),
        (100, 82, 0, "part", '<span class="vz-review-percent">82%</span>'),
        (100, 99, 0, "part", '<span class="vz-review-percent">99%</span>'),
        (100, 100, 4, "stale", '<span class="vz-review-percent">100%</span> \u00b7 <span class="vz-review-changed">'),
    ],
)
def test_the_badge_colours_its_percent_green_only_when_all_is_read(book, lines, read, stale, tone, inner):
    write(book, {"tracks.md": {"lines": lines, "read": read, "stale": stale, "reviewed": "2026-03-02"}})
    html = render(PAGE)
    assert (
        f'<span class="vz-review-badge vz-sidebar-badge" data-vz-review-tone="{tone}" '
        f'title="Last read and signed 2 Mar 2026">Read {inner}'
    ) in html


def test_no_badge_when_nothing_is_read(book):
    write(book, {"tracks.md": {"lines": 10, "read": 0, "stale": 3, "sections": []}})
    html = render(PAGE)
    assert "vz-review" not in html


def test_marks_wrap_each_run_and_its_bar_says_when_it_was_read(book):
    html = render(PAGE)
    assert (
        '<div class="vz-review-run" data-vz-review="read">\n'
        '<span class="vz-review-bar" title="Read 26 Nov 2026"></span>\n<p>Under the title.</p>\n</div>'
    ) in html
    assert (
        '<div class="vz-review-run" data-vz-review="partial">\n'
        '<span class="vz-review-bar" title="Partly read, last 20 Nov 2026"></span>\n'
        '<h2 id="items-and-tracks">Items and tracks</h2>\n<p>An item.</p>\n<ul>'
    ) in html
    assert (
        '<div class="vz-review-run" data-vz-review="stale">\n'
        '<span class="vz-review-bar" title="Changed since read"></span>\n<h3 id="tape-format">Tape format</h3>'
    ) in html
    # unread: no mark, outside any run; the title in none
    assert '</div>\n<h2 id="open-questions">Open questions</h2>\n<p>Nobody read this.</p>' in html
    assert html.count('<div class="vz-review-run"') == 3
    assert 'run" data-vz-review="read">\n<span class="vz-review-bar" title="Read 26 Nov 2026"></span>\n<h1' not in html


def test_a_page_without_marks_has_a_plain_badge(book):
    write(book, {"tracks.md": {"lines": 10, "read": 10, "stale": 0, "sections": []}})
    html = render(PAGE)
    assert '<span class="vz-review-badge vz-sidebar-badge"' in html
    assert "vz-review-toggle" not in html and "vz-review-run" not in html


def test_with_line_runs_each_block_takes_the_state_of_its_own_lines(book):
    (book / "docs").mkdir()
    (book / "docs" / "tracks.md").write_text(PAGE)
    runs = [
        {"from": 1, "to": 7, "state": "read", "reviewed": "2026-11-26"},
        {"from": 9, "to": 9, "state": "stale", "reviewed": "2026-11-20"},
        {"from": 13, "to": 13, "state": "read", "reviewed": "2026-11-20"},
    ]
    write(book, {"tracks.md": {"lines": 9, "read": 5, "stale": 1, "reviewed": "2026-11-26", "runs": runs}})
    html = render(PAGE)
    assert (
        '<div class="vz-review-run" data-vz-review="read">\n'
        '<span class="vz-review-bar" title="Read and signed 26 Nov 2026"></span>\n<h1 id="tracks">'
    ) in html
    assert '<p>An item.</p>\n</div>\n<div class="vz-review-run" data-vz-review="stale">' in html
    assert '</ul>\n</div>\n<h3 id="tape-format">Tape format</h3>' in html, "an unread heading has no mark"
    assert 'title="Read and signed 20 Nov 2026"></span>\n<p>Changed since.</p>\n</div>' in html
    assert html.count('<div class="vz-review-run"') == 3


def test_a_source_that_splits_otherwise_falls_back_to_sections(book):
    (book / "docs").mkdir()
    (book / "docs" / "tracks.md").write_text(PAGE + "\nOne more.\n")
    runs = [{"from": 1, "to": 17, "state": "read"}]
    write(
        book,
        {
            "tracks.md": {
                "lines": 9,
                "read": 9,
                "stale": 0,
                "runs": runs,
                "sections": [{"id": None, "line": 1, "state": "partial"}],
            }
        },
    )
    html = render(PAGE)
    assert 'data-vz-review="partial"' in html and 'data-vz-review="read"' not in html


def test_a_snippet_counts_the_blocks_it_includes(book):
    (book / "docs").mkdir()
    (book / "plot.txt").write_text("| a |\n|---|\n| 1 |\n\nA caption.\n")
    page = '# Plot\n\n--8<-- "plot.txt"\n\nAfter.\n'
    (book / "docs" / "tracks.md").write_text(page)
    runs = [{"from": 1, "to": 3, "state": "read"}]
    write(book, {"tracks.md": {"lines": 3, "read": 2, "stale": 0, "runs": runs}})
    rendered = page.replace('--8<-- "plot.txt"', (book / "plot.txt").read_text())
    html = render(rendered)
    assert html.count("vz-review-bar") == 1
    assert "<p>A caption.</p>\n</div>\n<p>After.</p>" in html


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


def test_blocks_above_the_title_take_no_mark(book):
    html = render("<div>hero</div>\n\n" + PAGE)
    assert html.startswith('<button aria-pressed="false" class="vz-review-badge')
    assert "<div>hero" in html and 'read">\n<div>hero' not in html and 'Read and signed 26 Nov 2026"></span>\n<div>hero' not in html
    assert 'title="Read 26 Nov 2026"></span>\n<p>Under the title.</p>' in html
