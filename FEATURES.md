# Features

Status: ✅ done · 🚧 in progress · 📋 planned · 🐛 bug. Ids are stable; never renumber or reuse.

## REFS - glossary, code links and cards

- 🚧 `REFS-1` book-refs: glossary terms `[[term]]`, code links `` [`Item`] `` and file links in any Zensical book, each with a card; `vz refs` writes the code index and the cards' files; an unknown name fails the build. The Markdown extensions, cards and index are done; the `vz refs` command is K4. Successor of spirit_level BOOK-7 book-refs.
- ✅ `REFS-2` code-card-links: a card's head links its lines on GitHub at the book's commit; crates in `on_crates_io` link to docs.rs. Successor of spirit_level DOCS-16 code-card-links.
- ✅ `REFS-3` book-formats: `vz_core` holds the formats the kit and spl's lints share: glossary.toml, docs/abbreviations.toml, the extensions' ids and settings (`ExtConfig`), the code index types with `FORMAT`, the stamp's markers; shared fixtures in `vz_core/tests/fixtures`.
- ✅ `REFS-4` code-index: `vz_refs` indexes a workspace's public items and git's text files; own cargo loader, no spirit_level. Matches spl 0.8.0's index of ww_tape and logring.
- ✅ `REFS-5` card-assets: the cards' `stylesheets/vz-refs.css` and `javascripts/vz-refs.js` (`vz-` classes, `data-vz-card`) and the code colours `stylesheets/vz-code.css`; node tests of the script.
- ✅ `REFS-6` abbreviations: the `vhrd_zensical.abbreviations` extension shows an abbreviation's long form on hover from `docs/abbreviations.toml`, parsed with `vz_core`'s format; the `abbr` extension must be on.

## STAMP - build stamp

- 📋 `STAMP-1` book-stamp: the footer build stamp (crate version, git commit, build time) and the script that shows the build's age; `vz stamp`. Successor of spirit_level BOOK-3 book-stamp and BOOK-9 book-build-stamp.

## LEX - lexers

- ✅ `LEX-1` console-lexer: Pygments `vz-console` colours each command, subcommand and flag; `vz-rust` marks every macro call. Successor of spirit_level BOOK-8 console-lexer.

## PKG - package and command

- ✅ `PKG-1` one-wheel: the package `vhrd-zensical` carries the extension module `vhrd_zensical._vz` and the console script `vz` (`vz_cli`), one version; `vz` has `--version`, `--color`, completion and a self-registering command registry.
- ✅ `PKG-2` git-install-check: `just package-check` installs the package from git at HEAD in a fresh uv cache and checks `vz --version` and the import; CI runs it.
- 📋 `PKG-3` pypi-wheels: prebuilt wheels on PyPI, so a book needs no Rust toolchain.
