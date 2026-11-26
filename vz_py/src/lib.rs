//! The extension module `vhrd_zensical._vz`: the `vz` command and the Markdown extensions' logic inside the
//! Python package.
//!
//! The package's console script `vz` calls `main`, so `uv run --with vhrd-zensical vz ...` runs the same build of
//! `vz_cli` as the package's Markdown extensions, at one version. The extensions resolve their references and
//! build their cards through the module `refs`, and show a page's proofreading through `review`, both over
//! `vz_refs`.

mod refs;
mod review;

use pyo3::prelude::*;

/// Runs the `vz` command and returns its exit status.
///
/// `argv` is the whole command line, program name first, as `sys.argv`; without it `sys.argv` is used, which is
/// what the console script needs. The command runs with the GIL released.
#[pyfunction]
#[pyo3(signature = (argv = None))]
fn main(py: Python<'_>, argv: Option<Vec<String>>) -> PyResult<u8> {
    let argv = match argv {
        Some(a) => a,
        None => py.import("sys")?.getattr("argv")?.extract()?,
    };
    Ok(py.detach(|| vz_cli::main(argv)).code())
}

/// The module `vhrd_zensical._vz`
#[pymodule]
#[pyo3(name = "_vz")]
fn vz_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(main, m)?)?;
    refs::register(m)?;
    review::register(m)
}
