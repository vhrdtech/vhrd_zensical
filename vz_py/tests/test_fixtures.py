"""The book formats' shared fixtures (`vz_core/tests/fixtures`), read by both the Rust and the Python parsers so
neither can drift from the other.
"""

from pathlib import Path

import pytest

from vhrd_zensical.abbreviations import AbbreviationError, _parse as parse_abbreviations
from vhrd_zensical.glossary import Glossary, GlossaryError

FIXTURES = Path(__file__).resolve().parents[2] / "vz_core" / "tests" / "fixtures"


def test_the_valid_glossary_parses():
    g = Glossary(FIXTURES / "glossary" / "ok.toml", "/glossary/")
    assert [t["name"] for t in g.terms] == ["CRC", "piece", "tape"]
    piece = g.find("Pieces")
    assert piece["name"] == "piece"
    assert piece["chapter"] == "../pieces/"
    crc = g.find("crc")
    assert crc["long"] == "cyclic redundancy check"


@pytest.mark.parametrize(
    "fixture",
    sorted(p.name for p in (FIXTURES / "glossary").glob("err-*.toml")),
)
def test_a_refused_glossary_fails(fixture):
    with pytest.raises(GlossaryError):
        Glossary(FIXTURES / "glossary" / fixture, "/glossary/")


def test_the_valid_abbreviations_file_parses():
    text = (FIXTURES / "abbreviations" / "ok.toml").read_text(encoding="utf-8")
    pairs = dict(parse_abbreviations(text))
    assert pairs["CAN"] == "Controller Area Network"
    assert pairs["I/O"] == "input/output"
    assert pairs["eMMC"] == "embedded MultiMediaCard"


@pytest.mark.parametrize(
    "fixture",
    sorted(p.name for p in (FIXTURES / "abbreviations").glob("err-*.toml")),
)
def test_a_refused_abbreviations_file_fails(fixture):
    text = (FIXTURES / "abbreviations" / fixture).read_text(encoding="utf-8")
    with pytest.raises(AbbreviationError):
        parse_abbreviations(text)
