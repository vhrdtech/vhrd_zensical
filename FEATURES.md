# Features

Status: ✅ done · 🚧 in progress · 📋 planned · 🐛 bug. Ids are stable; never renumber or reuse.

## REFS - glossary, code links and cards

- ✅ `REFS-1` book-refs: glossary terms `[[term]]`, code links `` [`Item`] `` and file links in any Zensical book, each with a card; `vz refs` checks the glossary and writes the code index and the cards' files; an unknown name fails the build. Successor of spirit_level BOOK-7 book-refs.
- ✅ `REFS-2` code-card-links: a card's head links its lines on GitHub at the book's commit; crates in `on_crates_io` link to docs.rs. Successor of spirit_level DOCS-16 code-card-links.
- ✅ `REFS-3` book-formats: `vz_core` holds the formats the kit and spl's lints share: glossary.toml, docs/abbreviations.toml, the extensions' ids and settings (`ExtConfig`), the code index types with `FORMAT`, the stamp's markers; shared fixtures in `vz_core/tests/fixtures`.
- ✅ `REFS-4` code-index: `vz_refs` indexes a workspace's public items and git's text files; own cargo loader, no spirit_level. Matches spl 0.8.0's index of ww_tape and logring.
- ✅ `REFS-5` card-assets: the cards' `stylesheets/vz-refs.css` and `javascripts/vz-refs.js` (`vz-` classes, `data-vz-card`) and the code colours `stylesheets/vz-code.css`; node tests of the script.
- ✅ `REFS-6` abbreviations: the `vhrd_zensical.abbreviations` extension shows an abbreviation's long form on hover from `docs/abbreviations.toml`, parsed with `vz_core`'s format; the `abbr` extension must be on.
- ✅ `REFS-7` assets-command: `vz assets` writes the cards' stylesheet and script and the code colours unconditionally, for a book that does not turn on `vz refs`'s checks.
- ✅ `REFS-8` refs-in-rust: the extensions' logic is Rust (`vz_refs`, through `vhrd_zensical._vz`): file loading, name resolution and its errors, docs.rs addresses, doc-comment links, text bounding, cards. Python keeps the Python-Markdown and Pygments adapters.
- ✅ `REFS-9` copied-page-refs: a book's sources file (`vz_core::sources`, `sources` setting) lists repositories it copies pages from; `vz refs` indexes each at its commit; links on a copied page resolve there first, then the book's own.

## STAMP - build stamp

- ✅ `STAMP-1` book-stamp: the footer build stamp (crate version, git commit, build time) and the script that shows the build's age; `vz stamp`. Successor of spirit_level BOOK-3 book-stamp and BOOK-9 book-build-stamp.

## LEX - lexers

- ✅ `LEX-1` console-lexer: Pygments `vz-console` colours each command, subcommand and flag; `vz-rust` marks every macro call. Successor of spirit_level BOOK-8 console-lexer.
- ✅ `LEX-2` rust-name-kinds: `vz-rust` colours names by shape as GitHub does: calls, upper-case types and constants; a struct literal's values are not types.

## FRAME - book frame

- ✅ `FRAME-1` frame-check: `vz check` checks the nav against `book_checks.toml`: ramp parts in level order, a level on every page, no page twice, bare `rust` fences within waived counts that only go down, no generated file left unincluded. Successor of wire_weaver's `ww-book check`.

## REVIEW - proofread display

- ✅ `REVIEW-1` proofread-display: the `vhrd_zensical.review` extension reads glossator's book export (`vz_core::review`, format 1; another format or no file shows nothing); a page badge `Proofread 82% · changed since` under the title, and per-section review marks the reader turns on with a button or the `r` key, kept in localStorage.

## PKG - package and command

- ✅ `PKG-1` one-wheel: the package `vhrd-zensical` carries the extension module `vhrd_zensical._vz` and the console script `vz` (`vz_cli`), one version; `vz` has `--version`, `--color`, completion and a self-registering command registry.
- ✅ `PKG-2` git-install-check: `just package-check` installs the package from git at HEAD in a fresh uv cache and checks `vz --version` and the import; CI runs it.
- 🚧 `PKG-3` pypi-wheels: prebuilt wheels on PyPI and GitHub releases, built by CI on a tag (`release.yml`), so a book needs no Rust toolchain. Open: the first tag run and the PyPI trusted publisher.
- ✅ `PKG-4` book-recipe: `vz prep` writes everything a book's settings ask for in one step; `vz build` and `vz serve` run it, then Zensical, from the same environment. The kit pins its tested Zensical as an extra, `vhrd-zensical[zensical]`, so a book's recipe pins one thing.
