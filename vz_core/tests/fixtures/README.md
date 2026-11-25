# Shared fixtures

The book formats' test files, read by the Rust tests of `vz_core` and the Python tests of the package, so both parsers accept and refuse the same files.

| File | Means |
|---|---|
| `glossary/ok.toml`, `abbreviations/ok.toml` | valid; the tests check what each parser reads from it |
| `<format>/err-<kind>[-case].toml` | refused with the error of that kind |
| `code_index.json` | the code index layout's snapshot, `format` 3: the book and one source |
| `sources/ok.toml` | valid; the sources file layout's snapshot |

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
| sources | `parse` | TOML syntax, a missing key, a value of the wrong type |
| sources | `format` | a `format` other than 1 |
| sources | `name` | a name that is empty or holds other than letters, digits and `_.-` |
| sources | `two-sources` | two sources of one name |
| sources | `root` | an empty `root` |
| sources | `repository` | a repository that is no `https://` address |
| sources | `commit` | a commit that is no full hash |
| sources | `no-pages` | a source without pages |
| sources | `page` | a page that is no relative path under the docs folder |
| sources | `page-twice` | a page under two sources |
