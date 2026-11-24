//! Where on its page a failing reference is written, for its error.

use std::path::Path;

/// Bytes of a page read at most to find a reference's line
// spirit-level: allow(exact-units-quantity) a byte count passed to take(), next to std's u64 sizes
const PAGE_BYTES_MAX: u64 = 16 << 20;

/// `page:line` of the first line holding `text`
///
/// `page` is the page's path as Zensical names it, relative to the docs folder `docs`; its file is read from
/// `docs/page`, else `page`. Without a page, or when neither reads, the lines are `source`, the Markdown given;
/// the name is then `page` as given, or `the page`. Without a line holding `text`, the name alone.
pub fn locate(page: Option<&str>, docs: &str, source: &[String], text: &str) -> String {
    let mut name = page.unwrap_or_default().to_owned();
    let mut read = None;
    if let Some(page) = page {
        for f in [Path::new(docs).join(page), Path::new(page).to_path_buf()] {
            if let Ok(t) = crate::read::text(&f, PAGE_BYTES_MAX) {
                name = f.display().to_string();
                read = Some(t);
                break;
            }
        }
    }
    let line = match &read {
        Some(t) => t.split('\n').position(|l| l.contains(text)),
        None => source.iter().position(|l| l.contains(text)),
    };
    if name.is_empty() {
        "the page".clone_into(&mut name);
    }
    match line {
        Some(n) => format!("{name}:{}", n + 1),
        None => name,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_the_line_of_the_markdown_given() {
        let source = [
            "First.".to_owned(),
            String::new(),
            "See [`Gone`].".to_owned(),
        ];
        assert_eq!(locate(None, "docs", &source, "[`Gone`]"), "the page:3");
        assert_eq!(locate(None, "docs", &source, "absent"), "the page");
        assert_eq!(
            locate(Some("/nonexistent/x.md"), "docs", &source, "[`Gone`]"),
            "/nonexistent/x.md:3"
        );
    }

    #[test]
    fn reads_the_page_from_the_docs_folder() {
        let dir = std::env::temp_dir().join(format!("vz-page-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("crashes.md"),
            "---\ntitle: x\n---\n\nSee [`Gone`].\n",
        )
        .unwrap();
        let docs = dir.display().to_string();
        assert_eq!(
            locate(Some("crashes.md"), &docs, &[], "[`Gone`]"),
            format!("{docs}/crashes.md:5")
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
