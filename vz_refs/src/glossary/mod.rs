//! The glossary as a book shows it: a term's link and card, and the glossary page's entries.
//!
//! `[[term]]` in prose links to the term's entry on the glossary page, `{page}#term-{slug}`, and shows its card on
//! hover. The glossary page holds the line [`PAGE_MARKER`], which becomes every entry in order of name. With the
//! `abbr` extension on, every page also gets each abbreviation's long form ([`crate::abbreviations`]).

use crate::cards::{self, Render, escape};
use crate::read::FileError;
use std::path::Path;
use vz_core::glossary::{Glossary, GlossaryError, Term};

/// The line of the glossary page that becomes its entries
pub const PAGE_MARKER: &str = "<!-- glossary -->";

/// A book's glossary with the address of its page
#[derive(Clone, Debug)]
pub struct BookGlossary {
    glossary: Glossary,
    page: String,
    /// The terms' indices in order of name, without regard to case
    by_name: Vec<usize>,
}

/// A term a reference names
#[derive(Clone, Debug)]
pub struct TermRef {
    /// What its card is told apart by on a page
    pub key: String,
    /// Its entry on the glossary page
    pub href: String,
    term: Term,
}

/// A reference that names no term
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnknownTerm {
    /// The word as written
    pub form: String,
}

impl std::fmt::Display for UnknownTerm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[[{}]]: no such term in the glossary", self.form)
    }
}

impl std::error::Error for UnknownTerm {}

impl BookGlossary {
    /// The glossary at `path`, named `file` in errors, whose page is served at `page`
    ///
    /// # Errors
    ///
    /// The file cannot be read or is no glossary.
    pub fn read(path: &Path, file: &str, page: &str) -> Result<Self, FileError<GlossaryError>> {
        crate::read::format(path, file, Glossary::parse).map(|g| Self::new(g, page))
    }

    /// `glossary`, whose page is served at `page`
    pub fn new(glossary: Glossary, page: &str) -> Self {
        let mut by_name: Vec<usize> = (0..glossary.terms.len()).collect();
        by_name.sort_by_cached_key(|&i| glossary.terms[i].name.to_lowercase());
        Self {
            glossary,
            page: page.to_owned(),
            by_name,
        }
    }

    /// The terms in order of name, without regard to case
    pub fn terms(&self) -> impl Iterator<Item = &Term> {
        self.by_name.iter().map(|&i| &self.glossary.terms[i])
    }

    /// The term `form` names
    ///
    /// # Errors
    ///
    /// No term has this name or form.
    pub fn find(&self, form: &str) -> Result<TermRef, UnknownTerm> {
        let term = self.glossary.find(form).ok_or_else(|| UnknownTerm {
            form: form.to_owned(),
        })?;
        Ok(TermRef {
            key: format!("term {}", term.name),
            href: format!("{}#term-{}", self.page, cards::slug(&term.name)),
            term: term.clone(),
        })
    }

    /// The card of `term`, with the id `card_id`: its name, long form and definition
    ///
    /// # Errors
    ///
    /// The renderer fails.
    pub fn card<R: Render>(card_id: &str, term: &TermRef, render: &R) -> Result<String, R::Error> {
        let t = &term.term;
        let head = match &t.long {
            Some(long) => format!("{} ({long})", t.name),
            None => t.name.clone(),
        };
        let body = format!(
            "<strong>{}</strong>: {}",
            escape(&head),
            render.inline(&t.short)?
        );
        Ok(cards::frame(card_id, "term", &body))
    }

    /// A page's lines with each [`PAGE_MARKER`] line made the glossary's entries; with `abbr`, every
    /// abbreviation's long form appended as an `abbr` definition
    ///
    /// # Errors
    ///
    /// The renderer fails.
    pub fn expand<R: Render>(
        &self,
        lines: Vec<String>,
        render: &R,
        abbr: bool,
    ) -> Result<Vec<String>, R::Error> {
        let mut out = Vec::with_capacity(lines.len());
        for line in lines {
            if line.trim() != PAGE_MARKER {
                out.push(line);
                continue;
            }
            for t in self.terms() {
                out.push(entry(t, render)?);
                out.push(String::new());
            }
        }
        if abbr {
            let definitions: Vec<String> = self
                .terms()
                .filter_map(|t| {
                    t.long
                        .as_deref()
                        .map(|long| crate::abbreviations::definition(&t.name, long))
                })
                .collect();
            if !definitions.is_empty() {
                out.push(String::new());
                out.extend(definitions);
            }
        }
        Ok(out)
    }
}

/// A term's entry on the glossary page, with its anchor and a link to its chapter
fn entry<R: Render>(t: &Term, render: &R) -> Result<String, R::Error> {
    let mut name = escape(&t.name);
    if let Some(long) = &t.long {
        name = format!("{name} ({})", escape(long));
    }
    let more = t
        .chapter
        .as_deref()
        .map(|c| format!(r#" <a href="{}">More</a>"#, escape(c)))
        .unwrap_or_default();
    Ok(format!(
        r#"<p class="vz-gloss" id="term-{}"><strong>{name}</strong>: {}{more}</p>"#,
        cards::slug(&t.name),
        render.inline(&t.short)?
    ))
}

#[cfg(test)]
mod tests;
