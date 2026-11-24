# vz_core

The book kit's formats: glossary, abbreviations, extension settings, the code index schema and the stamp's markers; types and parsers, no I/O.

Layer `core`: depends on nothing else in the workspace. spirit_level takes this crate by git for its book lints, so it stays small.

| Module | Format |
|---|---|
| `glossary` | `docs/glossary.toml`: terms, their forms, short definitions, chapters, long forms |
| `abbreviations` | `docs/abbreviations.toml`: `ABBR = "long form"` |
| `ext` | the extensions' ids in `markdown_extensions`, their settings and defaults, read through `ExtConfig` |
| `code_index` | the code index's JSON types and `FORMAT` |
| `stamp` | the footer build stamp's classes, markers and default paths |

`tests/fixtures` holds valid and refused files of each format; the package's Python tests read them too (see its README).
