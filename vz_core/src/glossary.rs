//! The book's glossary file, `docs/glossary.toml` by default: one entry per term.
//!
//! ```toml
//! [[term]]
//! name = "piece"                    # as the glossary page lists it
//! forms = ["pieces"]                # other words prose uses for it: plurals, other forms
//! short = "One file of a tape: ..." # one sentence, shown on hover
//! chapter = "../pieces/"            # the page that explains it ("More" on the glossary page)
//!
//! [[term]]
//! name = "CRC"
//! long = "cyclic redundancy check"  # an abbreviation's long form, shown on hover at every use
//! short = "A checksum ..."
//! ```
//!
//! A word names one term at most: a form listed under two entries is an error, so a reference cannot be read two
//! ways. Forms match without regard to case and are never blank. A `chapter` is a relative link to a page of the
//! book (letters, digits and `._~%#/-`, not starting with `/`): no scheme, so it cannot run script or leave the
//! site.

use crate::FILE_BYTES_MAX;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::collections::btree_map::Entry;

/// One entry
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Term {
    /// The term as the glossary page lists it
    pub name: String,
    /// Other words prose uses for it
    #[serde(default)]
    pub forms: Vec<String>,
    /// Its definition in one sentence
    pub short: String,
    /// The page that explains it, as a relative link
    #[serde(default)]
    pub chapter: Option<String>,
    /// For an abbreviation, its long form
    #[serde(default)]
    pub long: Option<String>,
}

impl Term {
    /// Its name and its other forms
    pub fn all_forms(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.name.as_str()).chain(self.forms.iter().map(String::as_str))
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    #[serde(default)]
    term: Vec<Term>,
}

/// Why a text is no glossary
#[derive(Debug)]
pub enum GlossaryError {
    /// The text is longer than [`FILE_BYTES_MAX`]
    TooLarge,
    /// TOML syntax, a missing or unknown key, a value of the wrong type
    Parse(toml::de::Error),
    /// A name or form that is blank
    BlankForm {
        /// The term, as its name is written
        term: String,
    },
    /// A `chapter` that is no relative link to a page
    Chapter {
        /// The term
        term: String,
        /// The chapter as written
        chapter: String,
    },
    /// A word names two terms
    TwoTerms {
        /// The word
        form: String,
        /// The term listed first
        first: String,
        /// The other term
        second: String,
    },
}

impl std::fmt::Display for GlossaryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooLarge => write!(f, "a glossary is {FILE_BYTES_MAX} bytes at most"),
            Self::Parse(e) => write!(f, "{e}"),
            Self::BlankForm { term } => write!(f, "term \"{term}\": a name or form is blank"),
            Self::Chapter { term, chapter } => write!(
                f,
                "term \"{term}\": chapter \"{chapter}\" is no relative link to a page of the book, such as \"../pieces/\""
            ),
            Self::TwoTerms {
                form,
                first,
                second,
            } => write!(
                f,
                "\"{form}\" names two terms, \"{first}\" and \"{second}\""
            ),
        }
    }
}

impl std::error::Error for GlossaryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Parse(e) => Some(e),
            Self::TooLarge
            | Self::BlankForm { .. }
            | Self::Chapter { .. }
            | Self::TwoTerms { .. } => None,
        }
    }
}

/// The terms of a book
#[derive(Clone, Debug, Default)]
pub struct Glossary {
    /// The entries, in file order
    pub terms: Vec<Term>,
    /// Each form, trimmed and lowercased, and the index of its term
    by_form: BTreeMap<String, usize>,
}

impl Glossary {
    /// Parses a glossary
    ///
    /// # Errors
    ///
    /// The text is too large or no glossary, a form is blank, a chapter is no relative page link, or a word in it
    /// names two terms.
    pub fn parse(text: &str) -> Result<Self, GlossaryError> {
        if text.len() > FILE_BYTES_MAX {
            return Err(GlossaryError::TooLarge);
        }
        let file: File = toml::from_str(text).map_err(GlossaryError::Parse)?;
        // each form's term: its index and its name
        let mut by_form: BTreeMap<String, (usize, &str)> = BTreeMap::new();
        for (i, t) in file.term.iter().enumerate() {
            if let Some(chapter) = &t.chapter
                && !is_relative_page(chapter)
            {
                return Err(GlossaryError::Chapter {
                    term: t.name.clone(),
                    chapter: chapter.clone(),
                });
            }
            for form in t.all_forms() {
                let key = key(form);
                if key.is_empty() {
                    return Err(GlossaryError::BlankForm {
                        term: t.name.clone(),
                    });
                }
                match by_form.entry(key) {
                    Entry::Vacant(v) => {
                        v.insert((i, &t.name));
                    }
                    Entry::Occupied(o) if o.get().0 == i => {}
                    Entry::Occupied(o) => {
                        return Err(GlossaryError::TwoTerms {
                            form: form.to_owned(),
                            first: o.get().1.to_owned(),
                            second: t.name.clone(),
                        });
                    }
                }
            }
        }
        let by_form = by_form.into_iter().map(|(k, (i, _))| (k, i)).collect();
        Ok(Self {
            terms: file.term,
            by_form,
        })
    }

    /// The term a word names, without regard to case or surrounding space
    pub fn find(&self, form: &str) -> Option<&Term> {
        self.by_form
            .get(&key(form))
            .and_then(|&i| self.terms.get(i))
    }
}

/// The key a form is found by
fn key(form: &str) -> String {
    form.trim().to_lowercase()
}

/// Whether `link` is a relative link to a page: no scheme, no host, no absolute path
fn is_relative_page(link: &str) -> bool {
    !link.is_empty()
        && !link.starts_with('/')
        && link
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "._~%#/-".contains(c))
}

#[cfg(test)]
mod tests {
    use super::*;

    const TWO: &str = "[[term]]\nname = \"piece\"\nforms = [\"pieces\"]\nshort = \"One file.\"\nchapter = \"../pieces/\"\n\n[[term]]\nname = \"CRC\"\nlong = \"cyclic redundancy check\"\nshort = \"A checksum.\"\n";

    #[test]
    fn finds_a_term_by_any_form_and_case() {
        let g = Glossary::parse(TWO).unwrap();
        assert_eq!(g.find("Pieces").unwrap().name, "piece");
        assert_eq!(g.find(" piece ").unwrap().name, "piece");
        let crc = g.find("crc").unwrap();
        assert_eq!(crc.long.as_deref(), Some("cyclic redundancy check"));
        assert_eq!(crc.chapter, None);
        assert_eq!(crc.forms.len(), 0);
        assert!(g.find("chain").is_none());
    }

    #[test]
    fn keeps_every_key_as_written() {
        let g = Glossary::parse(TWO).unwrap();
        assert_eq!(
            g.terms[0],
            Term {
                name: "piece".to_owned(),
                forms: vec!["pieces".to_owned()],
                short: "One file.".to_owned(),
                chapter: Some("../pieces/".to_owned()),
                long: None,
            }
        );
        assert_eq!(
            g.terms[0].all_forms().collect::<Vec<_>>(),
            ["piece", "pieces"]
        );
    }

    #[test]
    fn an_empty_text_has_no_terms() {
        assert_eq!(Glossary::parse("").unwrap().terms.len(), 0);
    }

    #[test]
    fn a_form_repeated_within_its_own_term_is_fine() {
        let g = Glossary::parse("[[term]]\nname = \"CRC\"\nforms = [\"crc\"]\nshort = \"x\"\n")
            .unwrap();
        assert_eq!(g.find("Crc").unwrap().name, "CRC");
    }

    #[test]
    fn a_word_naming_two_terms_is_an_error() {
        let text =
            format!("{TWO}\n[[term]]\nname = \"slice\"\nforms = [\"Piece\"]\nshort = \"x\"\n");
        let e = Glossary::parse(&text).unwrap_err();
        assert_eq!(
            e.to_string(),
            "\"Piece\" names two terms, \"piece\" and \"slice\""
        );
    }

    #[test]
    fn a_blank_form_is_an_error() {
        for text in [
            "[[term]]\nname = \" \"\nshort = \"x\"\n",
            "[[term]]\nname = \"a\"\nforms = [\"\"]\nshort = \"x\"\n",
        ] {
            let e = Glossary::parse(text).unwrap_err();
            assert!(matches!(e, GlossaryError::BlankForm { .. }), "{text}: {e}");
        }
    }

    #[test]
    fn a_chapter_is_a_relative_page_link() {
        for bad in [
            "javascript:alert(1)",
            "https://x.org/",
            "//x.org/a",
            "/abs/",
            "a b",
            "",
        ] {
            let text = format!("[[term]]\nname = \"a\"\nshort = \"b\"\nchapter = \"{bad}\"\n");
            let e = Glossary::parse(&text).unwrap_err();
            assert!(e.to_string().contains("is no relative link"), "{bad}: {e}");
        }
        for good in ["../pieces/", "pieces.md#crc", "a_b~c%20/"] {
            let text = format!("[[term]]\nname = \"a\"\nshort = \"b\"\nchapter = \"{good}\"\n");
            assert!(Glossary::parse(&text).is_ok(), "{good}");
        }
    }

    #[test]
    fn an_unknown_or_missing_key_is_an_error() {
        let e =
            Glossary::parse("[[term]]\nname = \"a\"\nshort = \"b\"\nshrot = \"c\"\n").unwrap_err();
        assert!(e.to_string().contains("unknown field"), "{e}");
        let e = Glossary::parse("[[term]]\nname = \"a\"\n").unwrap_err();
        assert!(e.to_string().contains("missing field `short`"), "{e}");
        let e = Glossary::parse("[[term]]\nshort = \"a\"\n").unwrap_err();
        assert!(e.to_string().contains("missing field `name`"), "{e}");
        let e = Glossary::parse("title = \"x\"\n").unwrap_err();
        assert!(e.to_string().contains("unknown field"), "{e}");
    }

    #[test]
    fn a_value_of_the_wrong_type_or_bad_syntax_is_an_error() {
        for text in [
            "[[term]]\nname = \"a\"\nshort = \"b\"\nforms = \"c\"\n",
            "[[term]]\nname = 1\nshort = \"b\"\n",
            "[[term]]\nname = \"a\"\nshort = \"b\"\nlong = []\n",
            "[[term]\n",
        ] {
            let e = Glossary::parse(text).unwrap_err();
            assert!(matches!(e, GlossaryError::Parse(_)), "{text}: {e}");
            assert!(std::error::Error::source(&e).is_some());
        }
    }

    #[test]
    fn a_text_over_the_limit_is_refused_before_parsing() {
        let text = " ".repeat(FILE_BYTES_MAX + 1);
        let e = Glossary::parse(&text).unwrap_err();
        assert!(matches!(e, GlossaryError::TooLarge));
        assert!(Glossary::parse(&" ".repeat(FILE_BYTES_MAX)).is_ok());
    }
}
