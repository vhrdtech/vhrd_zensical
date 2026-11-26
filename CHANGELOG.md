# Changelog

All notable changes to this project are recorded here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

## [0.4.0] - 2026-11-26

### Added

- `vhrd_zensical.review`: a read badge beside the table of contents and per-block review marks toggled with `r` (REVIEW-1).

### Changed

- Code comments in `vz-code.css` keep their muted colour and lose the italic (REFS-5).

### Fixed

- `vz-rust` colours calls, upper-case types and constants; struct literal values and `true` are no longer types (LEX-2).

## [0.3.1] - 2026-11-25

### Added

- `vz check`: a book's nav level order, page levels, duplicate pages, bare `rust` fences, stale generated files (FRAME-1).

## [0.3.0] - 2026-11-25

### Added

- Code links on pages copied from other repositories resolve against those repositories, named in a sources file (REFS-9).

### Changed

- **Breaking:** the code index is format 3: the book's workspace under `book`, each source's under `sources` (REFS-9).
- Ambiguity and unknown-file errors name where they looked: `the book's crates`, `the book's repository` (REFS-9).

## [0.2.1] - 2026-11-24

### Fixed

- A code span of 2+ backticks no longer crashes the build on Markdown 3.7-3.10 (REFS-1).

## [0.2.0] - 2026-11-24

### Added

- `release.yml`: on a tag, builds abi3 wheels and an sdist, checks them, publishes to GitHub and PyPI (PKG-3).

### Changed

- The extensions' logic moved into Rust behind `vhrd_zensical._vz`; Python keeps only the Markdown and Pygments adapters (REFS-8).
- `vz refs` reads the glossary bounded, through the same loader as the extension (REFS-8).

### Fixed

- A code link naming an item's whole path resolves to it, so every form an ambiguity error suggests works (REFS-8).

## [0.1.0] - 2026-11-24

### Added

- Python package `vhrd-zensical` carrying the `vz` command as a console script, one version (PKG-1).
- `vz` with `--version`, `--color` and shell completion; commands register themselves (PKG-1).
- `vz_core`: glossary.toml parser, moved from spirit_level sl_book_refs; forms are never blank (REFS-3).
- `docs/abbreviations.toml`: abbreviations with long forms, replacing spirit_level.toml's `long-forms` table (REFS-3).
- Extension ids `vhrd_zensical.glossary`, `.code_refs`, `.abbreviations`; `on_crates_io` is a code_refs setting (REFS-3).
- Code index types with `FORMAT` 2, the stamp's markers and default paths, shared format fixtures (REFS-3).
- `vz_refs`: code index of a workspace's public items and tracked text files, with its own cargo loader (REFS-4).
- Card assets `vz-refs.css`, `vz-refs.js` and code colours `vz-code.css`, renamed from spl's `spl-` classes (REFS-5).
- `just test-js`: the cards' script on a fake page under node, part of `just test` (REFS-5).
- `just package-check`: install from git at HEAD in a fresh uv cache, check `vz` and the import (PKG-2).
- Python package: glossary and code-link Markdown extensions with hover cards, moved from spirit_level's `spl_book_ext` (REFS-1, REFS-2).
- `vhrd_zensical.abbreviations`: reads `docs/abbreviations.toml` directly, replacing the JSON table spirit_level wrote (REFS-6).
- Pygments lexers `vz-console` and `vz-rust`, found through the package's entry points (LEX-1).
- `vz_core/tests/fixtures`: pytest reads the same glossary and abbreviations fixtures as the Rust parsers (REFS-3).
- `vz_stamp`: the footer build stamp and its age script, on `vz_core`'s markers and default paths (STAMP-1).
- `vz refs`: checks the glossary, writes the code index and the cards' stylesheet and script (REFS-1).
- `vz stamp`: writes the stamp partial under the theme's `custom_dir` and the age script (STAMP-1).
- `vz assets`: writes every card and code colour asset unconditionally (REFS-7).
- `vz prep`: writes everything a book's `zensical.toml` settings ask for in one step (PKG-4).
- `vz build` and `vz serve`: `vz prep`, then Zensical, from the same environment (PKG-4).
- `vhrd-zensical[zensical]`: an extra pinning the kit's tested Zensical release, 0.1.2 (PKG-4).
