"""The book formats' shared fixtures (`vz_core/tests/fixtures`), read through the extension module: each refused
file raises the extension's own error, which fails the build.
"""

from pathlib import Path

import pytest

from vhrd_zensical import _vz
from vhrd_zensical.abbreviations import AbbreviationError
from vhrd_zensical.glossary import GlossaryError

FIXTURES = Path(__file__).resolve().parents[2] / "vz_core" / "tests" / "fixtures"


def test_the_valid_glossary_reads():
    g = _vz.Glossary(str(FIXTURES / "glossary" / "ok.toml"), "/glossary/")
    assert g.find("Pieces").href == "/glossary/#term-piece"
    assert g.find("crc").key == "term CRC"


@pytest.mark.parametrize(
    "fixture",
    sorted(p.name for p in (FIXTURES / "glossary").glob("err-*.toml")),
)
def test_a_refused_glossary_fails(fixture):
    with pytest.raises(GlossaryError, match=f"^{FIXTURES}/glossary/{fixture}: "):
        _vz.Glossary(str(FIXTURES / "glossary" / fixture), "/glossary/")


def test_the_valid_abbreviations_file_reads():
    a = _vz.Abbreviations(str(FIXTURES / "abbreviations" / "ok.toml"))
    defs = a.define([])
    assert "*[CAN]: Controller Area Network" in defs
    assert "*[I/O]: input/output" in defs
    assert "*[eMMC]: embedded MultiMediaCard" in defs


@pytest.mark.parametrize(
    "fixture",
    sorted(p.name for p in (FIXTURES / "abbreviations").glob("err-*.toml")),
)
def test_a_refused_abbreviations_file_fails(fixture):
    with pytest.raises(AbbreviationError, match=f"^{FIXTURES}/abbreviations/{fixture}: "):
        _vz.Abbreviations(str(FIXTURES / "abbreviations" / fixture))
