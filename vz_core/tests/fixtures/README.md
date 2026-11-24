# Shared fixtures

The book formats' test files, read by the Rust tests of `vz_core` and the Python tests of the package, so both parsers accept and refuse the same files.

| File | Means |
|---|---|
| `glossary/ok.toml`, `abbreviations/ok.toml` | valid; the tests check what each parser reads from it |
| `<format>/err-<kind>[-case].toml` | refused with the error of that kind |
| `code_index.json` | the code index layout's snapshot, `format` 2 |

Error kinds:

| Format | Kind | Refused for |
|---|---|---|
| glossary | `parse` | TOML syntax, a missing or unknown key, a value of the wrong type |
| glossary | `blank-form` | a name or form that is blank |
| glossary | `chapter` | a chapter that is no relative page link |
| glossary | `two-terms` | a word naming two terms, without regard to case |
| abbreviations | `parse` | TOML syntax, a value that is no string |
| abbreviations | `abbreviation` | an abbreviation that is empty, has surrounding space, spans lines or holds `[` or `]` |
| abbreviations | `long-form` | a long form that is blank or spans lines |
