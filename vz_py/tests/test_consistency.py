"""Every class the extensions write is styled by `vz_refs`' stylesheet (moved from spirit_level's sl_book_refs)."""

from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[2]
EXTENSION = [
    (ROOT / "vz_py" / "python" / "vhrd_zensical" / "card.py").read_text(encoding="utf-8"),
    (ROOT / "vz_py" / "python" / "vhrd_zensical" / "glossary.py").read_text(encoding="utf-8"),
    (ROOT / "vz_py" / "python" / "vhrd_zensical" / "code_refs.py").read_text(encoding="utf-8"),
]
CSS_PATH = ROOT / "vz_refs" / "assets" / "refs.css"


@pytest.mark.skipif(not CSS_PATH.exists(), reason="vz_refs/assets/refs.css not landed yet (K3)")
def test_the_stylesheet_styles_every_class_the_extension_writes():
    py = "".join(EXTENSION)
    css = CSS_PATH.read_text(encoding="utf-8")
    for cls in [
        "vz-ref",
        "vz-card",
        "vz-card-head",
        "vz-card-src",
        "vz-card-doc",
        "vz-card-where",
        "vz-card-links",
        "vz-card-more",
        "vz-cards",
        "vz-gloss",
    ]:
        assert f'"{cls}' in py, f"the extension writes no {cls}"
        assert f".{cls}" in css, f"the stylesheet lacks .{cls}"
    for kind in ["term", "code", "file"]:
        assert f'"{kind}", card_id)' in py, f"the extension writes no vz-{kind}"
        assert f".vz-{kind}" in css, f"the stylesheet lacks .vz-{kind}"
