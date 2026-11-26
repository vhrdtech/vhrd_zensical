# vz_core

The book kit's formats: glossary, abbreviations, extension settings, the frame file, the code index schema, the review file and the stamp's markers; types and parsers, no I/O.

Layer `core`: depends on nothing else in the workspace. spirit_level takes this crate by git for its book lints, so it stays small.

| Module | Format |
|---|---|
| `glossary` | `docs/glossary.toml`: terms, their forms, short definitions, chapters, long forms |
| `abbreviations` | `docs/abbreviations.toml`: `ABBR = "long form"` |
| `ext` | the extensions' ids in `markdown_extensions`, their settings and defaults, read through `ExtConfig` |
| `code_index` | the code index's JSON types and `FORMAT` |
| `frame` | `book_checks.toml`: the reading ramp, shelves and waived fences, and the frame check over the nav and pages |
| `review` | the review file (`glossator book export`, JSON): each page's proofread lines, last day and section states; another format parses to nothing |
| `stamp` | the footer build stamp's classes, markers and default paths |

`tests/fixtures` holds valid and refused files of each format; the package's Python tests read them too, through the extension module, to check each refusal raises the extension's error.
