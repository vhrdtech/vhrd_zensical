# Changelog

All notable changes to this project are recorded here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

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
