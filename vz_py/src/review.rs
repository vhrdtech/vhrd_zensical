//! The review extension's logic: the review file read, a page's proofreading badge and its blocks' review marks,
//! over `vz_refs::review`. The extension keeps only the walk over Python-Markdown's tree.

use crate::refs::RefError;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use std::path::Path;
use vz_refs::review::Block;

pyo3::create_exception!(
    _vz,
    ReviewError,
    RefError,
    "A review file that does not read"
);

/// The book's review file, read from `file`; empty when it is missing or of a format this kit does not know
#[pyclass(frozen, module = "vhrd_zensical._vz")]
pub(crate) struct Review(Option<vz_core::review::Review>);

/// One page's proofreading
#[pyclass(frozen, module = "vhrd_zensical._vz")]
pub(crate) struct ReviewPage(vz_core::review::Page);

#[pymethods]
impl Review {
    #[new]
    fn new(file: &str) -> PyResult<Self> {
        vz_refs::review::read(Path::new(file), file)
            .map(Self)
            .map_err(|e| ReviewError::new_err(e.to_string()))
    }

    /// The proofreading of `page`, relative to the docs folder; `None` when the file has none
    fn page(&self, page: &str) -> Option<ReviewPage> {
        self.0.as_ref()?.page(page).cloned().map(ReviewPage)
    }
}

#[pymethods]
impl ReviewPage {
    /// The badge under the title, `(text, tooltip)`; `None` when no line is proofread
    #[getter]
    fn badge(&self) -> Option<(String, String)> {
        vz_refs::review::badge(&self.0).map(|b| (b.text, b.title))
    }

    /// The mark of each top-level block, `read`, `stale`, `partial` or `None`; a block is `("title", None)`,
    /// `("heading", id)` or `("other", None)`
    #[expect(
        clippy::needless_pass_by_value,
        reason = "pyo3 extracts a Python list as an owned Vec"
    )]
    fn marks(&self, blocks: Vec<(String, Option<String>)>) -> PyResult<Vec<Option<&'static str>>> {
        let blocks = blocks
            .iter()
            .map(|(kind, id)| match kind.as_str() {
                "title" => Ok(Block::Title),
                "heading" => Ok(Block::Heading(id.as_deref())),
                "other" => Ok(Block::Other),
                k => Err(PyValueError::new_err(format!(
                    "a block is title, heading or other, not {k:?}"
                ))),
            })
            .collect::<PyResult<Vec<_>>>()?;
        Ok(vz_refs::review::marks(&self.0, &blocks))
    }
}

/// Adds the review classes, their error and the default file to the module
pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("ReviewError", m.py().get_type::<ReviewError>())?;
    m.add_class::<Review>()?;
    m.add_class::<ReviewPage>()?;
    m.add("DEFAULT_REVIEW", vz_core::ext::DEFAULT_REVIEW)
}
