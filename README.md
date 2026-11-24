# vhrd_zensical

[![spirit_level](.github/spl-badge.svg)](https://spl.vhrd.tech/tiers/small-workspace/)
[![CI](https://github.com/vhrdtech/vhrd_zensical/actions/workflows/ci.yml/badge.svg)](https://github.com/vhrdtech/vhrd_zensical/actions/workflows/ci.yml)

Zensical book kit: code cards, glossary, abbreviations, lexers, build stamp.

One Python package, `vhrd-zensical`, carries the Markdown extensions and lexers and the `vz` command, built from the same Rust workspace. A book pins one version of the kit and gets all of them at that version.

## Use in a book

A release publishes prebuilt wheels to PyPI and the GitHub release (CPython 3.12+; Linux x86_64 and aarch64, glibc and musl; macOS x86_64 and arm64 in one universal2 wheel; Windows x86_64). A book pins the wheel, needs no Rust toolchain, and gets the Zensical release the kit is tested against with the `zensical` extra:

```just
kit := "vhrd-zensical[zensical]==0.1.0"

book:
    uv run --with "{{ kit }}" vz --version
```

The same pin works in a book's `pyproject.toml`: `dependencies = ["vhrd-zensical[zensical]==0.1.0"]`.

An unreleased commit still installs from git; the first `uv run` of a new commit builds the Rust part (about 1-2 minutes; needs a Rust toolchain), then uv caches it:

```just
kit := "vhrd-zensical @ git+https://github.com/vhrdtech/vhrd_zensical@<sha>"
```

Status of each part: [FEATURES.md](FEATURES.md).

## Layout

| Crate | What |
|---|---|
| `vz_core` | the book formats: types and parsers, no I/O |
| `vz_refs` | the code index and the code cards' assets |
| `vz_stamp` | the footer build stamp |
| `vz_cli` | the `vz` command |
| `vz_py` | the extension module `vhrd_zensical._vz` and the Python package (`vz_py/python/vhrd_zensical`) |

## Licence

Either of [`LICENSE-MIT`](LICENSE-MIT) or [`LICENSE-APACHE`](LICENSE-APACHE), at your option.
