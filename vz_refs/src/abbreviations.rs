//! The book's abbreviations as `abbr` definitions on every page.
//!
//! Each abbreviation of the abbreviations file becomes a definition of Python-Markdown's `abbr` extension,
//! `*[CAN]: Controller Area Network`, appended to each page; the extension then marks every use in the page's
//! text as `<abbr title="...">`. They come after the glossary's own long forms, so the file wins over a term's
//! `long`.

use crate::read::FileError;
use std::path::Path;
use vz_core::abbreviations::{Abbreviations, AbbreviationsError};

/// The abbreviations file at `path`, named `file` in errors
///
/// # Errors
///
/// The file cannot be read or is no abbreviations file.
pub fn read(path: &Path, file: &str) -> Result<Abbreviations, FileError<AbbreviationsError>> {
    crate::read::format(path, file, Abbreviations::parse)
}

/// A page's lines with every abbreviation's definition appended, after a blank line
pub fn define(abbreviations: &Abbreviations, mut lines: Vec<String>) -> Vec<String> {
    if abbreviations.long_forms.is_empty() {
        return lines;
    }
    lines.push(String::new());
    lines.extend(
        abbreviations
            .long_forms
            .iter()
            .map(|(abbr, long)| definition(abbr, long)),
    );
    lines
}

/// The `abbr` extension's definition of one abbreviation
pub(crate) fn definition(abbr: &str, long: &str) -> String {
    format!("*[{abbr}]: {long}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appends_a_definition_per_abbreviation() {
        let a =
            Abbreviations::parse("SD = \"Secure Digital\"\nCAN = \"Controller Area Network\"\n")
                .unwrap();
        assert_eq!(
            define(&a, vec!["Text.".to_owned()]),
            [
                "Text.",
                "",
                "*[CAN]: Controller Area Network",
                "*[SD]: Secure Digital"
            ]
        );
        assert_eq!(
            define(&Abbreviations::default(), vec!["x".to_owned()]),
            ["x"]
        );
    }

    #[test]
    fn a_missing_or_bad_file_names_the_file() {
        let e = read(Path::new("/nonexistent/abbr.toml"), "abbr.toml").unwrap_err();
        assert!(e.to_string().starts_with("abbr.toml: "), "{e}");
    }
}
