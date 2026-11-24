import markdown
import pytest

from vhrd_zensical.abbreviations import AbbreviationError

TABLE = 'CAN = "Controller Area Network"\nSD = "Secure Digital"\n"Q&A" = "say \\"x\\" <b>"\n'


@pytest.fixture
def table(tmp_path, monkeypatch):
    monkeypatch.chdir(tmp_path)
    (tmp_path / "abbreviations.toml").write_text(TABLE)
    (tmp_path / "glossary.toml").write_text('[[term]]\nname = "SD"\nlong = "glossary wording"\nshort = "A card."\n')
    return tmp_path


def render(text, abbr=True, glossary=False):
    exts = [*(["abbr"] if abbr else []), "vhrd_zensical.abbreviations", *(["vhrd_zensical.glossary"] if glossary else [])]
    return markdown.markdown(
        text,
        extensions=exts,
        extension_configs={
            "vhrd_zensical.abbreviations": {"file": "abbreviations.toml"},
            "vhrd_zensical.glossary": {"file": "glossary.toml"},
        },
    )


def test_every_use_shows_its_long_form(table):
    html = render("The CAN bus and an SD card. CAN again.")
    assert html.count('<abbr title="Controller Area Network">CAN</abbr>') == 2
    assert '<abbr title="Secure Digital">SD</abbr>' in html
    assert "*[" not in html


def test_code_keeps_the_abbreviation_as_written(table):
    html = render("A `CAN` frame.\n\n```text\nCAN SD\n```\n")
    assert "<abbr" not in html


def test_a_word_containing_it_is_left_alone(table):
    assert "<abbr" not in render("The CANDLE and SDK.")


def test_the_long_form_is_text(table):
    html = render("Ask in the Q&A.")
    assert "<b>" not in html and "&lt;b&gt;" in html


def test_the_table_wins_over_the_glossary(table):
    html = render("An SD card.", glossary=True)
    assert '<abbr title="Secure Digital">SD</abbr>' in html


def test_without_abbr_the_build_fails(table):
    with pytest.raises(AbbreviationError, match="needs the abbr extension"):
        render("CAN", abbr=False)


def test_a_missing_file_names_the_file(table):
    (table / "abbreviations.toml").unlink()
    with pytest.raises(AbbreviationError):
        render("CAN")


def test_a_malformed_file_fails(table):
    (table / "abbreviations.toml").write_text('"a[b" = "x"\n')
    with pytest.raises(AbbreviationError, match="no abbreviation"):
        render("x")
