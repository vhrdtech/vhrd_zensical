# vz_py

The extension module `vhrd_zensical._vz`: the `vz` command for the Python package's console script, built by maturin.

- `python/vhrd_zensical/`: the Python package `vhrd-zensical` (import `vhrd_zensical`).
- The root `pyproject.toml` builds it: `vz = "vhrd_zensical._vz:main"`, so one install gives the extensions and the `vz` command at the same version.
- `tests/`: pytest, run by `just test` through uv.

Layer `app`. No `unsafe`: pyo3's macros compile under the workspace's `unsafe_code = "forbid"`.
