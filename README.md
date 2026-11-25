# vhrd_zensical

[![spirit_level](.github/spl-badge.svg)](https://spl.vhrd.tech/tiers/small-workspace/)
[![CI](https://github.com/vhrdtech/vhrd_zensical/actions/workflows/ci.yml/badge.svg)](https://github.com/vhrdtech/vhrd_zensical/actions/workflows/ci.yml)

Zensical book kit: code cards, glossary, abbreviations, lexers, build stamp, frame check.

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

## Pages copied from other repositories

A book that copies pages from other repositories' checkouts gets their code links resolved against those repositories. Its assembler writes a sources file at every build (not kept in git), and the code link extension names it:

```toml
[project.markdown_extensions]
"vhrd_zensical.code_refs" = { crates = ["wire_weaver"], sources = "target/vz/sources.toml" }
```

```toml
format = 1

[[source]]
name = "ww_stdlib"                                    # unique
root = "/home/u/.cache/ww_book/ww_stdlib"             # the checkout: git root and cargo workspace root; absolute or relative to the project root
repository = "https://github.com/vhrdtech/ww_stdlib"
commit = "0123456789abcdef0123456789abcdef01234567"   # full hash checked out at root
crates = ["ww_logic_analyzer"]                        # library names; empty or left out: every library
on_crates_io = ["ww_logic_analyzer"]                  # package names: their cards link to docs.rs
pages = ["std_library/ww_logic_analyzer.md"]          # the copies, relative to the docs folder
```

- `vz refs` (and `vz prep`) indexes each source's workspace at its commit, beside the book's own, into the one code index.
- A link on a copied page resolves against its source's crates and files first, then the book's own; its card links the source's repository at that commit.
- A name found in neither fails the build and names both places: `` [`Gone`]: no public item of ww_stdlib's or the book's crates has this name ``.
- An index written before the sources file changed fails until `vz prep` runs again; a listed page missing from the docs folder fails `vz refs`.
- The format's reference: `vz_core::sources`; its snapshot `vz_core/tests/fixtures/sources/ok.toml`. Readers ignore keys they do not know.

## The book's frame

`vz check` checks a book's nav and pages against `book_checks.toml` at the project root:

```toml
outside = ["Reference", "Articles"]  # top-level nav sections after the ramp; their pages may have any level

[[part]]                             # the reading ramp, in nav order
title = "Basic"                      # a top-level nav section
level = "basic"                      # the level of every page in it: basic, advanced or nomicon

[bare_fences]                        # pages still holding bare `rust` fences, under the docs folder
"guide/old.md" = 3                   # how many it may hold; the count may only go down
```

It fails:

- a ramp that steps down a level when the reader clicks next, or a page in a part of another level;
- a top-level section that is neither a part nor a shelf in `outside`, or a part after a shelf;
- a page without `level:` in its front matter, a page listed twice, a page not in the docs folder;
- a `rust` fence that is not an include of a compiled source (`--8<-- "path:section"`), beyond the page's waived count; a Nomicon page may also show a sketch, marked `title="sketch: ..."`;
- a waived count higher than the page's bare fences, so counts only go down; a copied page (one the sources file lists) may hold fewer;
- a file under `<docs>/generated/` that no page includes.

Pages are read from the docs folder: a book that copies pages assembles them before `vz check`. The format's reference: `vz_core::frame`.

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
