"""Every class the Python side writes is styled by `vz_refs`' stylesheet; the cards' own classes are checked by
`vz_refs::cards`' tests."""

from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CARD = (ROOT / "vz_py" / "python" / "vhrd_zensical" / "card.py").read_text(encoding="utf-8")
EXTENSIONS = "".join(
    (ROOT / "vz_py" / "python" / "vhrd_zensical" / f).read_text(encoding="utf-8")
    for f in ["glossary.py", "code_refs.py"]
)
CSS = (ROOT / "vz_refs" / "assets" / "vz-refs.css").read_text(encoding="utf-8")


def test_the_stylesheet_styles_every_class_the_extensions_write():
    for cls in ["vz-ref", "vz-cards"]:
        assert f'"{cls}' in CARD, f"card.py writes no {cls}"
        assert f".{cls}" in CSS, f"the stylesheet lacks .{cls}"
    for kind in ["term", "code", "file"]:
        assert f'"{kind}", card_id)' in EXTENSIONS, f"the extensions write no vz-{kind}"
        assert f".vz-{kind}" in CSS, f"the stylesheet lacks .vz-{kind}"
