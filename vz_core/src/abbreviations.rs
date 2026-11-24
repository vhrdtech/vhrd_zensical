//! The book's abbreviations file, `docs/abbreviations.toml` by default: each abbreviation with its long form.
//!
//! ```toml
//! CAN = "Controller Area Network"
//! CRC = "cyclic redundancy check"
//! eMMC = "embedded MultiMediaCard"
//! "I/O" = "input/output"
//! ```
//!
//! The abbreviations extension shows the long form on hover wherever a page writes the abbreviation; the
//! `abbreviation-first-use` lint of `spl` takes every abbreviation listed here as explained. An abbreviation
//! matches with its case: `CAN` and `can` are different words.
//!
//! An abbreviation is one line without surrounding space and holds no `[` or `]` (Python-Markdown's `abbr`
//! definitions cannot carry them). A long form is one line and not blank.

use crate::FILE_BYTES_MAX;
use std::collections::BTreeMap;

/// Why a text is no abbreviations file
#[derive(Debug)]
pub enum AbbreviationsError {
    /// The text is longer than [`FILE_BYTES_MAX`]
    TooLarge,
    /// TOML syntax, or a value that is no string
    Parse(toml::de::Error),
    /// An abbreviation that is empty, has surrounding space, spans lines or holds `[` or `]`
    Abbreviation {
        /// The abbreviation as written
        abbreviation: String,
    },
    /// A long form that is blank or spans lines
    LongForm {
        /// The abbreviation
        abbreviation: String,
        /// The long form as written
        long: String,
    },
}

impl std::fmt::Display for AbbreviationsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooLarge => write!(f, "an abbreviations file is {FILE_BYTES_MAX} bytes at most"),
            Self::Parse(e) => write!(f, "{e}"),
            Self::Abbreviation { abbreviation } => write!(
                f,
                "{abbreviation:?} is no abbreviation: one line, no surrounding space, no `[` or `]`"
            ),
            Self::LongForm { abbreviation, long } => write!(
                f,
                "\"{abbreviation}\": long form {long:?} is blank or spans lines"
            ),
        }
    }
}

impl std::error::Error for AbbreviationsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Parse(e) => Some(e),
            Self::TooLarge | Self::Abbreviation { .. } | Self::LongForm { .. } => None,
        }
    }
}

/// The abbreviations of a book with their long forms
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Abbreviations {
    /// Each abbreviation and its long form, sorted by abbreviation
    pub long_forms: BTreeMap<String, String>,
}

impl Abbreviations {
    /// Parses an abbreviations file
    ///
    /// # Errors
    ///
    /// The text is too large, not TOML, has a value that is no string, or an abbreviation or long form that breaks
    /// the rules above.
    pub fn parse(text: &str) -> Result<Self, AbbreviationsError> {
        if text.len() > FILE_BYTES_MAX {
            return Err(AbbreviationsError::TooLarge);
        }
        let long_forms: BTreeMap<String, String> =
            toml::from_str(text).map_err(AbbreviationsError::Parse)?;
        for (abbreviation, long) in &long_forms {
            let bad_char = |c: char| matches!(c, '[' | ']' | '\n' | '\r');
            if abbreviation.is_empty()
                || abbreviation.trim() != abbreviation
                || abbreviation.contains(bad_char)
            {
                return Err(AbbreviationsError::Abbreviation {
                    abbreviation: abbreviation.clone(),
                });
            }
            if long.trim().is_empty() || long.contains(['\n', '\r']) {
                return Err(AbbreviationsError::LongForm {
                    abbreviation: abbreviation.clone(),
                    long: long.clone(),
                });
            }
        }
        Ok(Self { long_forms })
    }

    /// The long form of an abbreviation, matched with its case
    pub fn long_form(&self, abbreviation: &str) -> Option<&str> {
        self.long_forms.get(abbreviation).map(String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_each_long_form_with_its_case() {
        let a = Abbreviations::parse(
            "CAN = \"Controller Area Network\"\neMMC = \"embedded MultiMediaCard\"\n\"I/O\" = \"input/output\"\n",
        )
        .unwrap();
        assert_eq!(a.long_form("CAN"), Some("Controller Area Network"));
        assert_eq!(a.long_form("I/O"), Some("input/output"));
        assert_eq!(a.long_form("can"), None);
        assert_eq!(
            a.long_forms.keys().collect::<Vec<_>>(),
            ["CAN", "I/O", "eMMC"]
        );
    }

    #[test]
    fn an_empty_text_has_no_abbreviations() {
        assert_eq!(Abbreviations::parse("").unwrap(), Abbreviations::default());
    }

    #[test]
    fn a_bad_abbreviation_is_an_error() {
        for key in ["\"\"", "\" CAN\"", "\"a[b\"", "\"a]b\"", "\"a\\nb\""] {
            let e = Abbreviations::parse(&format!("{key} = \"x\"\n")).unwrap_err();
            assert!(
                matches!(e, AbbreviationsError::Abbreviation { .. }),
                "{key}: {e}"
            );
            assert!(std::error::Error::source(&e).is_none());
        }
    }

    #[test]
    fn a_bad_long_form_is_an_error() {
        for long in ["\"\"", "\"  \"", "\"a\\nb\"", "\"a\\rb\""] {
            let e = Abbreviations::parse(&format!("CAN = {long}\n")).unwrap_err();
            assert!(
                matches!(e, AbbreviationsError::LongForm { .. }),
                "{long}: {e}"
            );
        }
        let e = Abbreviations::parse("CAN = \"\"\n").unwrap_err();
        assert_eq!(
            e.to_string(),
            "\"CAN\": long form \"\" is blank or spans lines"
        );
    }

    #[test]
    fn a_value_that_is_no_string_or_bad_syntax_is_an_error() {
        for text in [
            "CAN = 1\n",
            "[long-forms]\nCAN = \"x\"\n",
            "CAN = [\"x\"]\n",
            "CAN = \"x\"\nCAN = \"y\"\n",
            "CAN =\n",
        ] {
            let e = Abbreviations::parse(text).unwrap_err();
            assert!(matches!(e, AbbreviationsError::Parse(_)), "{text}: {e}");
            assert!(std::error::Error::source(&e).is_some());
        }
    }

    #[test]
    fn a_text_over_the_limit_is_refused_before_parsing() {
        let e = Abbreviations::parse(&" ".repeat(FILE_BYTES_MAX + 1)).unwrap_err();
        assert!(matches!(e, AbbreviationsError::TooLarge), "{e}");
        assert!(e.to_string().contains("bytes at most"));
    }
}
