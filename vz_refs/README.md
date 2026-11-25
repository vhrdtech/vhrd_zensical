# vz_refs

The code index of a book and everything its references show: a slim cargo workspace loader, the index over syn, code links and glossary terms resolved, their cards assembled, and the cards' assets.

Layer `model`: depends on `vz_core` only.

| Module | What |
|---|---|
| `workspace` | `cargo metadata --no-deps`, the members' library targets, their `.rs` files, `git ls-files` and `git rev-parse HEAD`; every program runs without an outer cargo's `CARGO_*` variables |
| `code_index` | every public item of a library's module tree (`mod x;`, `#[path]`, `pub use` re-exports followed; `#[cfg(test)]` modules left out) and every tracked text file with its `--8<--` sections, as `vz_core::code_index::Index` |
| `links` | code links resolved through the index, a copied page's against its source first (a whole path wins over suffix matches), intra-doc links in docs, docs bounded in length, docs.rs addresses, item and file cards |
| `sources` | the sources file read, each source's workspace indexed at its commit (its pages checked, its root the workspace root) |
| `glossary` | a term's link and card, the glossary page's entries, the long forms as `abbr` definitions |
| `abbreviations` | `docs/abbreviations.toml` as `abbr` definitions on every page |
| `cards` | card ids, frame and parts; `Render`, the Markdown renderer the caller brings (Python-Markdown and Pygments in the package) |
| `page` | the page and line a failing reference names |
| `read` | a book's files read whole, bounded |
| `assets` | `stylesheets/vz-refs.css`, `javascripts/vz-refs.js` (the cards) and `stylesheets/vz-code.css` (the code colours), relative to the docs folder |

`tests/projects/refs` is the fixture workspace the index tests read; `tests/projects/book` is a fixture book copying a page from `tests/projects/la`; `tests/vz-refs.test.mjs` tests the cards' script on a fake page (`just test-js`, needs node).
