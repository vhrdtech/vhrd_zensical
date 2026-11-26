"""A book's references in prose, as Python-Markdown extensions that Zensical loads from zensical.toml, plus the
``vz`` command's console script.

- ``vhrd_zensical.glossary``: ``[[term]]`` links a glossary term to its entry and shows its definition on hover.
- ``vhrd_zensical.code_refs``: `` [`Name`] `` links a public item of the book's crates and shows its signature and
  docs.
- ``vhrd_zensical.abbreviations``: an abbreviation of ``docs/abbreviations.toml`` shows its long form on hover
  (``<abbr>``).
- ``vhrd_zensical.review``: a page's proofreading from the book's review file: a badge under the title, and review
  marks beside its sections that the reader turns on with a button or the ``r`` key.
- ``vhrd_zensical.shell``: the ``console`` fence's lexer (alias ``vz-console``): commands, subcommands and flags in
  their own colours.
- ``vhrd_zensical.rust``: the ``rust`` fence's lexer (alias ``vz-rust``): every macro call marked, not only the
  standard ones.

Both draw the same card (classes ``vz-ref``, ``vz-card``), styled by the stylesheet ``vz assets`` writes. A card
opens on hover and on focus, so a tap or the keyboard opens it too. A name that resolves to nothing, or to several
things, raises an error that fails the build.
"""

from importlib.metadata import version as _version

__version__ = _version("vhrd-zensical")
