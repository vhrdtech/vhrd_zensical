# vz_py

The extension module `vhrd_zensical._vz`: the `vz` command and the Markdown extensions' logic, built by maturin.

- `python/vhrd_zensical/`: the Python package `vhrd-zensical` (import `vhrd_zensical`). It holds only what Zensical needs in Python: the extensions' registration with Python-Markdown (inline, tree and preprocessors), the two Pygments lexers, and the renderer of a card's Markdown. Loading the book's files, resolving names, their errors and the cards are `vz_refs`, through `_vz` (`src/refs.rs`).
- The root `pyproject.toml` builds it: `vz = "vhrd_zensical._vz:main"`, so one install gives the extensions and the `vz` command at the same version.
- `tests/`: pytest, run by `just test` through uv.

Layer `app`. No `unsafe`: pyo3's macros compile under the workspace's `unsafe_code = "forbid"`.
