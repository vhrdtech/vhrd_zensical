//! The references' logic for the Markdown extensions: the glossary, the code index, the abbreviations and the
//! cards, over `vz_refs`. The extensions keep only what Python-Markdown and Pygments need: registering their
//! processors and rendering a card's Markdown, which they pass in as `render`.

use pyo3::exceptions::PyException;
use pyo3::prelude::*;
use std::path::Path;
use vz_refs::cards::Render;
use vz_refs::glossary::{BookGlossary, TermRef};
use vz_refs::links::{CardSizes, CodeLinks, FileCardError, FileRef, ItemLink, ItemRef};

pyo3::create_exception!(
    _vz,
    RefError,
    PyException,
    "A reference that cannot be resolved, or a book file that does not read; it fails the build"
);
pyo3::create_exception!(
    _vz,
    GlossaryError,
    RefError,
    "A glossary that does not read, or a term it lacks"
);
pyo3::create_exception!(
    _vz,
    CodeRefError,
    RefError,
    "A code index that does not read, or a code link naming nothing or several things"
);
pyo3::create_exception!(
    _vz,
    AbbreviationError,
    RefError,
    "An abbreviations file that does not read"
);

/// The caller's renderer: a Python function from Markdown to safe HTML
struct PyRender<'a, 'py>(&'a Bound<'py, PyAny>);

impl Render for PyRender<'_, '_> {
    type Error = PyErr;

    fn markdown(&self, text: &str) -> PyResult<String> {
        self.0.call1((text,))?.extract()
    }
}

/// A book's glossary, read from `file`, whose page is served at `page`
#[pyclass(frozen, module = "vhrd_zensical._vz")]
pub(crate) struct Glossary(BookGlossary);

/// A term a reference names
#[pyclass(frozen, module = "vhrd_zensical._vz")]
pub(crate) struct Term(TermRef);

#[pymethods]
impl Term {
    /// What its card is told apart by on a page
    #[getter]
    fn key(&self) -> &str {
        &self.0.key
    }

    /// Its entry on the glossary page
    #[getter]
    fn href(&self) -> &str {
        &self.0.href
    }
}

#[pymethods]
impl Glossary {
    #[new]
    fn new(file: &str, page: &str) -> PyResult<Self> {
        BookGlossary::read(Path::new(file), file, page)
            .map(Self)
            .map_err(|e| GlossaryError::new_err(e.to_string()))
    }

    /// The term `form` names; `GlossaryError` when none
    fn find(&self, form: &str) -> PyResult<Term> {
        self.0
            .find(form)
            .map(Term)
            .map_err(|e| GlossaryError::new_err(e.to_string()))
    }

    /// The card of `term` with the id `card_id`, its definition rendered by `render`
    #[staticmethod]
    fn card(card_id: &str, term: &Term, render: &Bound<'_, PyAny>) -> PyResult<String> {
        BookGlossary::card(card_id, &term.0, &PyRender(render))
    }

    /// A page's lines with the glossary marker made the entries; with `abbr`, the long forms appended
    fn expand(
        &self,
        lines: Vec<String>,
        render: &Bound<'_, PyAny>,
        abbr: bool,
    ) -> PyResult<Vec<String>> {
        self.0.expand(lines, &PyRender(render), abbr)
    }
}

/// The code index at `file`, as code links read it
#[pyclass(frozen, module = "vhrd_zensical._vz")]
pub(crate) struct CodeIndex(CodeLinks);

/// The item a code link names
#[pyclass(frozen, module = "vhrd_zensical._vz")]
pub(crate) struct Item(ItemRef);

/// The file or section a file link names
#[pyclass(frozen, module = "vhrd_zensical._vz")]
pub(crate) struct File(FileRef);

macro_rules! ref_getters {
    ($t:ty) => {
        #[pymethods]
        impl $t {
            /// What its card is told apart by on a page
            #[getter]
            fn key(&self) -> &str {
                &self.0.key
            }

            /// Its lines on the forge
            #[getter]
            fn href(&self) -> &str {
                &self.0.href
            }

            /// What the link shows: the name as written
            #[getter]
            fn text(&self) -> &str {
                &self.0.text
            }
        }
    };
}

ref_getters!(Item);
ref_getters!(File);

fn code_ref_error(e: impl std::fmt::Display) -> PyErr {
    CodeRefError::new_err(e.to_string())
}

#[pymethods]
impl CodeIndex {
    #[new]
    fn new(file: &str) -> PyResult<Self> {
        CodeLinks::read(Path::new(file), file)
            .map(Self)
            .map_err(code_ref_error)
    }

    /// The one item a link names: `kind@`, `cfg(...)@`, the name and `()` or `!`; `CodeRefError` when none or
    /// several
    #[pyo3(signature = (name, kind = None, cfg = None, suffix = ""))]
    fn item(
        &self,
        name: &str,
        kind: Option<&str>,
        cfg: Option<&str>,
        suffix: &str,
    ) -> PyResult<Item> {
        let link = ItemLink {
            kind,
            cfg,
            name,
            suffix,
        };
        self.0.item(&link).map(Item).map_err(code_ref_error)
    }

    /// The file `name` names, or its section `section`; `CodeRefError` when none or several
    #[pyo3(signature = (name, section = None))]
    fn file(&self, name: &str, section: Option<&str>) -> PyResult<File> {
        self.0.file(name, section).map(File).map_err(code_ref_error)
    }

    /// The card of `item` with the id `card_id`, its code and docs rendered by `render`
    fn item_card(
        &self,
        card_id: &str,
        item: &Item,
        render: &Bound<'_, PyAny>,
        lines_max: usize,
        doc_chars_max: usize,
    ) -> PyResult<String> {
        let sizes = CardSizes {
            lines_max,
            doc_chars_max,
        };
        self.0.item_card(card_id, &item.0, sizes, &PyRender(render))
    }

    /// The card of `file` with the id `card_id`, its lines read from the working directory and rendered by
    /// `render`
    fn file_card(
        &self,
        card_id: &str,
        file: &File,
        render: &Bound<'_, PyAny>,
        lines_max: usize,
    ) -> PyResult<String> {
        self.0
            .file_card(
                card_id,
                &file.0,
                Path::new(""),
                lines_max,
                &PyRender(render),
            )
            .map_err(|e| match e {
                FileCardError::Render(e) => e,
                e @ FileCardError::Read { .. } => code_ref_error(e),
            })
    }
}

/// The book's abbreviations, read from `file`
#[pyclass(frozen, module = "vhrd_zensical._vz")]
pub(crate) struct Abbreviations(vz_core::abbreviations::Abbreviations);

#[pymethods]
impl Abbreviations {
    #[new]
    fn new(file: &str) -> PyResult<Self> {
        vz_refs::abbreviations::read(Path::new(file), file)
            .map(Self)
            .map_err(|e| AbbreviationError::new_err(e.to_string()))
    }

    /// A page's lines with every abbreviation's `abbr` definition appended
    fn define(&self, lines: Vec<String>) -> Vec<String> {
        vz_refs::abbreviations::define(&self.0, lines)
    }
}

/// The id of the card for `key`
#[pyfunction]
fn card_id(key: &str) -> String {
    vz_refs::cards::card_id(key)
}

/// `page:line` of the first line holding `text`: the page's file read from `docs`, else the lines `source`
#[pyfunction]
#[pyo3(signature = (page, docs, source, text))]
#[expect(
    clippy::needless_pass_by_value,
    reason = "pyo3 extracts a Python list as an owned Vec"
)]
fn locate(page: Option<&str>, docs: &str, source: Vec<String>, text: &str) -> String {
    vz_refs::page::locate(page, docs, &source, text)
}

/// Adds the classes, functions, exceptions and default settings to the module
pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    register_errors(m)?;
    register_classes(m)?;
    register_defaults(m)?;
    m.add_function(wrap_pyfunction!(card_id, m)?)?;
    m.add_function(wrap_pyfunction!(locate, m)?)
}

fn register_errors(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    m.add("RefError", py.get_type::<RefError>())?;
    m.add("GlossaryError", py.get_type::<GlossaryError>())?;
    m.add("CodeRefError", py.get_type::<CodeRefError>())?;
    m.add("AbbreviationError", py.get_type::<AbbreviationError>())
}

fn register_classes(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Glossary>()?;
    m.add_class::<Term>()?;
    m.add_class::<CodeIndex>()?;
    m.add_class::<Item>()?;
    m.add_class::<File>()?;
    m.add_class::<Abbreviations>()
}

/// The extensions' default settings, so the Python side states none of its own
fn register_defaults(m: &Bound<'_, PyModule>) -> PyResult<()> {
    use vz_core::ext;
    for (name, path) in [
        ("DEFAULT_GLOSSARY", ext::DEFAULT_GLOSSARY),
        ("DEFAULT_GLOSSARY_PAGE", ext::DEFAULT_GLOSSARY_PAGE),
        ("DEFAULT_CODE_INDEX", ext::DEFAULT_CODE_INDEX),
        ("DEFAULT_ABBREVIATIONS", ext::DEFAULT_ABBREVIATIONS),
    ] {
        m.add(name, path)?;
    }
    m.add("DEFAULT_CARD_LINES_MAX", ext::DEFAULT_CARD_LINES_MAX)?;
    m.add(
        "DEFAULT_CARD_DOC_CHARS_MAX",
        ext::DEFAULT_CARD_DOC_CHARS_MAX,
    )
}
