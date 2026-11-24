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
- `just package-check`: install from git at HEAD in a fresh uv cache, check `vz` and the import (PKG-2).
