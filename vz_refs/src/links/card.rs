//! The cards of code links: an item's kind, path, code and docs; a file's or section's lines.

use super::{CodeLinks, FileRef, ItemRef};
use crate::cards::{self, DOC, MORE, Render, SRC, WHERE, escape};
use std::path::Path;

/// How much a code card shows at most
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CardSizes {
    /// Lines of code
    pub lines_max: usize,
    /// Characters of docs, as Markdown; the first sentence always shows
    pub doc_chars_max: usize,
}

impl Default for CardSizes {
    fn default() -> Self {
        Self {
            lines_max: vz_core::ext::DEFAULT_CARD_LINES_MAX,
            doc_chars_max: vz_core::ext::DEFAULT_CARD_DOC_CHARS_MAX,
        }
    }
}

/// Why a file card cannot be written
#[derive(Debug)]
pub enum FileCardError<E> {
    /// The file cannot be read
    Read {
        /// Its path
        path: String,
        /// Why
        source: std::io::Error,
    },
    /// The renderer fails
    Render(E),
}

impl<E: std::fmt::Display> std::fmt::Display for FileCardError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Read { path, source } => write!(f, "{path}: {source}"),
            Self::Render(e) => write!(f, "{e}"),
        }
    }
}

impl<E: std::error::Error + 'static> std::error::Error for FileCardError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Read { source, .. } => Some(source),
            Self::Render(e) => Some(e),
        }
    }
}

/// The highlighting language of a file, by its extension
fn language(path: &str) -> String {
    let ext = path.rsplit('.').next().unwrap_or(path).to_lowercase();
    let lang = match ext.as_str() {
        "rs" => "rust",
        "py" => "python",
        "sh" => "bash",
        "js" => "javascript",
        "ts" => "typescript",
        "h" => "c",
        "yml" => "yaml",
        "md" => "markdown",
        "x" | "ld" => "text",
        other => other,
    };
    lang.to_owned()
}

impl CodeLinks {
    /// The card of `item`, with the id `card_id`: its kind and path, its code, its docs, where it is and its links
    ///
    /// # Errors
    ///
    /// The renderer fails.
    pub fn item_card<R: Render>(
        &self,
        card_id: &str,
        item: &ItemRef,
        sizes: CardSizes,
        render: &R,
    ) -> Result<String, R::Error> {
        let e = &item.entry;
        let part = &self.parts[item.part];
        let (src, left) = cards::first_lines(&e.source, sizes.lines_max);
        let arm = e
            .cfg
            .as_deref()
            .map(|c| format!(", cfg({c})"))
            .unwrap_or_default();
        let mut body = cards::head(&format!("{} {}{arm}", e.kind.name(), e.path), &e.url);
        body += &cards::part(SRC, &render.code(&src, "rust")?);
        body += &cards::more_lines(left);
        if !e.doc.is_empty() {
            let (doc, cut) = super::bounded(&part.doc_links(&e.doc, e), sizes.doc_chars_max);
            body += &cards::part(DOC, &render.markdown(&doc)?);
            if cut {
                body += &cards::part(MORE, "The docs go on in the source");
            }
        }
        body += &cards::part(WHERE, &escape(&format!("{}:{}", e.file, e.lines[0])));
        let file = e.url.split('#').next().unwrap_or(&e.url);
        let mut links = vec![
            ("Source on GitHub", e.url.as_str()),
            ("File on GitHub", file),
        ];
        let docs = part.docs_rs_url(e);
        if let Some(docs) = &docs {
            links.push(("docs.rs", docs));
        }
        body += &cards::links(&links);
        Ok(cards::frame(card_id, "code", &body))
    }

    /// The card of `file`, with the id `card_id`: its lines, read from its workspace under the project root `root`,
    /// where they are and their links
    ///
    /// # Errors
    ///
    /// The file cannot be read, or the renderer fails.
    pub fn file_card<R: Render>(
        &self,
        card_id: &str,
        file: &FileRef,
        root: &Path,
        lines_max: usize,
        render: &R,
    ) -> Result<String, FileCardError<R::Error>> {
        let part = &self.parts[file.part];
        let text = crate::read::text(
            &root.join(&part.root).join(&file.path),
            crate::workspace::FILE_BYTES_MAX,
        )
        .map_err(|source| FileCardError::Read {
            path: file.path.clone(),
            source,
        })?;
        let lines: Vec<&str> = text
            .split('\n')
            .skip(file.first.saturating_sub(1))
            .take((file.last + 1).saturating_sub(file.first))
            .collect();
        let (src, left) = cards::first_lines(&lines.join("\n"), lines_max);
        let head = match &file.section {
            Some(s) => format!("{}, section {s}", file.path),
            None => file.path.clone(),
        };
        let mut body = cards::head(&head, &file.href);
        body += &cards::part(
            SRC,
            &render
                .code(&src, &language(&file.path))
                .map_err(FileCardError::Render)?,
        );
        body += &cards::more_lines(left);
        body += &cards::part(
            WHERE,
            &escape(&format!("{}:{}-{}", file.path, file.first, file.last)),
        );
        let whole = format!("{}{}", part.index.blob, file.path);
        let mut links = Vec::new();
        if file.section.is_some() {
            links.push(("Lines on GitHub", file.href.as_str()));
        }
        links.push(("File on GitHub", whole.as_str()));
        body += &cards::links(&links);
        Ok(cards::frame(card_id, "file", &body))
    }
}
