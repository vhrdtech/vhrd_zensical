# Working on vhrd_zensical

Guidance for AI agents and contributors. Read it before changing code.

vhrd_zensical is a kit for Zensical books: glossary terms, code links and cards, abbreviations, Pygments lexers, a footer build stamp. One Python package, `vhrd-zensical`, carries the Markdown extensions, the lexers and the `vz` command, all built from this Rust workspace at one version. A book pins that one version.

## 1. The top rule: no stop-gap solutions

- Build the final architecture from the start. No shims, no "refactor later", no second path the final design throws away.
- A `TODO` that defers the right design is a defect; a `TODO(ID)` names a FEATURES.md item that is planned work.
- Until a release is in use, names and formats change freely: no aliases for an old id, class or path. Say the break in the CHANGELOG.

## 2. Terse output

Result first, then only what the reader must decide or do. Bullets for lists, tables for options, one fact per line. No recap, no narration. Limits: CHANGELOG entry 1 line and 20 words, commit subject 60 characters, this file 150 lines.

## 3. Layout

Crates sit at the repo root.

| Crate | Layer | What |
|---|---|---|
| `vz_core` | core | the book formats: glossary, abbreviations, extension settings, code index schema, stamp markers; types and parsers, no I/O |
| `vz_refs` | model | the code index: a slim cargo workspace loader, the index over syn, file entries, the cards' CSS and script |
| `vz_stamp` | model | the footer build stamp and its age script |
| `vz_cli` | app | the `vz` command: a library (`vz_cli::main`) and a thin binary |
| `vz_py` | app | the extension module `vhrd_zensical._vz` and the Python package in `vz_py/python/vhrd_zensical` |

The map in `spirit_level.toml` is the source of truth for layers; `spl check` enforces it.

- `vz_core` is also a dependency of spirit_level (its book lints read the formats through it, by git). Keep it small; a format change there needs a spl release too.
- `vz_core` does not know spirit_level's config: settings come through a small trait the caller implements.

## 4. The package: one wheel, one version

- The root `pyproject.toml` builds the package with maturin from `vz_py/Cargo.toml`; the Python sources live in `vz_py/python/`.
- The package version is the workspace version (`dynamic = ["version"]`). Never set a version in `pyproject.toml`.
- The console script `vz = "vhrd_zensical._vz:main"` runs `vz_cli::main`, the same code as the `vz` binary. A new command is Rust in `vz_cli`, never a Python entry point.
- `_vz.main(argv=None)` takes the whole command line, program name first; without it, `sys.argv`.
- Books install from git: `vhrd-zensical @ git+https://github.com/vhrdtech/vhrd_zensical@vX.Y.Z`. uv builds the Rust part on first use and caches it per commit.
- uv does not resolve `@HEAD` in a git URL (it looks for a branch or tag named `HEAD`): give a SHA or a tag. `just package-check` uses `$(git rev-parse HEAD)`.

## 5. Commands register themselves

- Each command group is a module of `vz_cli` that calls `inventory::submit!` with a `Registered` (clap builder API). Never a central enum or match.
- `registry::tests::registered_commands_are_the_expected_ones` lists every command: update it with each new one.
- Help, completion (`COMPLETE=bash vz`, `vc_complete`) and dispatch come from the same registry.
- Printing: clap prints help and usage errors; a command returns `Result<Exit, Box<dyn Error>>` and `vc_ui::finish` prints the one error line. Library code never prints.

## 6. Dependencies

- All in `[workspace.dependencies]`, `default-features = false`, only the features in use.
- No bundled C, no `-sys` crate with a native build. Check `cargo tree -e normal,build` when adding one; run `cargo tree -d` and `cargo machete`.
- `pyo3` and `pyo3-ffi` link the Python C API (`links = "python"`); nothing C is compiled. They are in spl's `no-sys-crates` allow list for that reason.
- `unsafe_code = "forbid"` holds for every crate, `vz_py` included: pyo3's `#[pymodule]` and `#[pyfunction]` expand to code that compiles under it (pyo3 0.30). No crate is marked unsafe in the map.
- pyo3 uses `abi3-py312`: one wheel for every CPython from 3.12.
- The CLI parts come from vhrd_cli by plain git (no `rev`): `vc_build` (`--version`), `vc_ui` (`--color`, error line, exit codes), `vc_complete`. `cargo update -p vc_ui` moves them.

## 7. FEATURES.md and CHANGELOG.md

- FEATURES.md is the only status tracker, ids per area (`REFS`, `STAMP`, `LEX`, `PKG`). Name ids with a slug when talking to people: `PKG-1 one-wheel`. New ids: `tpm work claim AREA --repo vhrd_zensical`.
- CHANGELOG.md: entries under `## [Unreleased]` (`### Added`, `### Changed`, `### Fixed`, `### Removed`), one line each, with the id.
- A branch never touches the version. A release bumps `[workspace.package] version` and tags `vX.Y.Z`; books pin the tag.
- Design notes stay out of this repo.

## 8. Commands

```bash
just                # the recipes
just test           # nextest, doctests, pytest (uv); `just test --changed` tests only what changed against main
just lint           # fmt, clippy -D warnings, cargo doc -D warnings, spl check
just package-check  # install from git at HEAD in a fresh uv cache: `vz --version` and `import vhrd_zensical`
just install        # build with the deploy profile, put vz on PATH
just jobs=4 test    # cap build and test parallelism for one run
```

## 9. Tests

- Rust: unit tests beside the code; one integration binary per crate (`tests/it/main.rs`). CLI tests run the `vz` binary with the outer `CARGO_*` variables removed.
- Python: `vz_py/tests`, pytest through uv (`just test-py`); uv builds the package into `.venv` first.
- Tests are deterministic and read no host state: fixtures and scratch directories only.
- `vz_core/tests/fixtures` holds valid and refused files of each book format; the Python tests read the same files. A format change updates both parsers and the fixtures together.
