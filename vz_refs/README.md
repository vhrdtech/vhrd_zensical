# vz_refs

The code index of a book: a slim cargo workspace loader, the index over syn, file entries and the code cards' assets.

Layer `model`: depends on `vz_core` only.

| Module | What |
|---|---|
| `workspace` | `cargo metadata --no-deps`, the members' library targets, their `.rs` files, `git ls-files` and `git rev-parse HEAD`; every program runs without an outer cargo's `CARGO_*` variables |
| `code_index` | every public item of a library's module tree (`mod x;`, `#[path]`, `pub use` re-exports followed; `#[cfg(test)]` modules left out) and every tracked text file with its `--8<--` sections, as `vz_core::code_index::Index` |
| `assets` | `stylesheets/vz-refs.css`, `javascripts/vz-refs.js` (the cards) and `stylesheets/vz-code.css` (the code colours), relative to the docs folder |

`tests/projects/refs` is the fixture workspace the index tests read; `tests/vz-refs.test.mjs` tests the cards' script on a fake page (`just test-js`, needs node).
