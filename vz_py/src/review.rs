//! The review extension's logic: the review file read, a page's proofreading badge and its blocks' review marks,
//! over `vz_refs::review`. The extension keeps only the walk over Python-Markdown's tree.

use crate::refs::RefError;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use std::collections::BTreeMap;
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
    /// The badge, `(text, tooltip, percent, stale)`; `None` when no line is proofread
    #[getter]
    fn badge(&self) -> Option<(String, String, u64, bool)> {
        vz_refs::review::badge(&self.0).map(|b| (b.text, b.title, b.percent, b.stale))
    }

    /// The mark of each top-level block, `(state, tooltip)` with state `read`, `stale` or `partial`, or `None`;
    /// a block is `("title", None)`,
    /// `("heading", id)` or `("other", None)`
    #[expect(
        clippy::needless_pass_by_value,
        reason = "pyo3 extracts a Python list as an owned Vec"
    )]
    fn marks(
        &self,
        blocks: Vec<(String, Option<String>)>,
    ) -> PyResult<Vec<Option<(&'static str, String)>>> {
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
        Ok(vz_refs::review::marks(&self.0, &blocks)
            .into_iter()
            .map(|m| m.map(|m| (m.state, m.title)))
            .collect())
    }

    /// The mark of each of the `count` top-level blocks `source` (the page's Markdown) renders to, from its read
    /// and stale lines; `None` when the source does not split into `count` blocks or the file has no lines
    #[expect(
        clippy::needless_pass_by_value,
        reason = "pyo3 extracts a Python dict as an owned map"
    )]
    fn block_marks(
        &self,
        source: &str,
        count: usize,
        included: BTreeMap<String, usize>,
    ) -> Option<Vec<Option<(&'static str, String)>>> {
        vz_refs::review::block_marks(&self.0, source, count, |p| included.get(p).copied()).map(
            |v| {
                v.into_iter()
                    .map(|m| m.map(|m| (m.state, m.title)))
                    .collect()
            },
        )
    }
}

/// The first and last line, 1-based, of each top-level block of a page's Markdown; `included`: the blocks of each
/// file a snippet line includes
#[pyfunction]
#[expect(
    clippy::needless_pass_by_value,
    reason = "pyo3 extracts a Python dict as an owned map"
)]
#[pyo3(signature = (source, included = BTreeMap::new()))]
fn md_blocks(source: &str, included: BTreeMap<String, usize>) -> Vec<(usize, usize)> {
    vz_refs::md_blocks::blocks_with(source, |p| included.get(p).copied())
}

/// Adds the review classes, their error and the default file to the module
pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("ReviewError", m.py().get_type::<ReviewError>())?;
    m.add_class::<Review>()?;
    m.add_class::<ReviewPage>()?;
    m.add_function(wrap_pyfunction!(md_blocks, m)?)?;
    m.add("DEFAULT_REVIEW", vz_core::ext::DEFAULT_REVIEW)
}
