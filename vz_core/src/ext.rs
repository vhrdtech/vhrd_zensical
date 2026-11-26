//! The kit's Markdown extensions in the book's configuration: their ids, settings and default paths.
//!
//! ```toml
//! [project.markdown_extensions]
//! "vhrd_zensical.glossary" = { file = "docs/glossary.toml", page = "/glossary/" }
//! "vhrd_zensical.code_refs" = { index = "target/vz/code_index.json", crates = ["ww_tape"], on_crates_io = ["ww-tape"], sources = "target/vz/sources.toml" }
//! "vhrd_zensical.abbreviations" = { file = "docs/abbreviations.toml" }
//! "vhrd_zensical.review" = { file = "target/vz/review.json" }
//! ```
//!
//! Paths are relative to the project root, where the book is built from. A setting left out takes its default
//! ([`DEFAULT_GLOSSARY`] and the others). The cards' sizes (`card_lines_max`, `card_doc_chars_max`) are read by the
//! extensions alone; their defaults are here too.
//!
//! The caller brings its configuration tree through [`ExtConfig`]: `vz` reads `zensical.toml` as a
//! [`toml::Table`], which implements it here; `spl` implements it over its own configuration node, which also
//! reads `mkdocs.yml`.

/// The glossary extension's id in `markdown_extensions`
pub const GLOSSARY_EXTENSION: &str = "vhrd_zensical.glossary";

/// The code link extension's id in `markdown_extensions`
pub const CODE_REFS_EXTENSION: &str = "vhrd_zensical.code_refs";

/// The abbreviations extension's id in `markdown_extensions`
pub const ABBREVIATIONS_EXTENSION: &str = "vhrd_zensical.abbreviations";

/// The review extension's id in `markdown_extensions`
pub const REVIEW_EXTENSION: &str = "vhrd_zensical.review";

/// The glossary file when the settings name none
pub const DEFAULT_GLOSSARY: &str = "docs/glossary.toml";

/// The glossary page's address when the settings name none
pub const DEFAULT_GLOSSARY_PAGE: &str = "/glossary/";

/// The code index when the settings name none: under `target/`, which a Rust repository already ignores
pub const DEFAULT_CODE_INDEX: &str = "target/vz/code_index.json";

/// The abbreviations file when the settings name none
pub const DEFAULT_ABBREVIATIONS: &str = "docs/abbreviations.toml";

/// The review file when the settings name none: written by the book before its build, under `target/`
pub const DEFAULT_REVIEW: &str = "target/vz/review.json";

/// Lines of code a code card shows at most when the settings name no `card_lines_max`
pub const DEFAULT_CARD_LINES_MAX: usize = 16;

/// Characters of docs a code card shows at most when the settings name no `card_doc_chars_max`; the first sentence
/// always shows
pub const DEFAULT_CARD_DOC_CHARS_MAX: usize = 400;

/// A setting that is there but of another type than the one asked for
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WrongType;

/// One extension's settings, as the caller's configuration holds them
pub trait ExtConfig {
    /// The string at `key`: `None` when the key is not there
    ///
    /// # Errors
    ///
    /// The value is no string.
    fn string(&self, key: &str) -> Result<Option<String>, WrongType>;

    /// The list of strings at `key`: `None` when the key is not there
    ///
    /// # Errors
    ///
    /// The value is no list, or an item of it is no string.
    fn strings(&self, key: &str) -> Result<Option<Vec<String>>, WrongType>;
}

impl ExtConfig for toml::Table {
    fn string(&self, key: &str) -> Result<Option<String>, WrongType> {
        self.get(key)
            .map(|v| v.as_str().map(str::to_owned).ok_or(WrongType))
            .transpose()
    }

    fn strings(&self, key: &str) -> Result<Option<Vec<String>>, WrongType> {
        self.get(key)
            .map(|v| {
                v.as_array()
                    .ok_or(WrongType)?
                    .iter()
                    .map(|i| i.as_str().map(str::to_owned).ok_or(WrongType))
                    .collect()
            })
            .transpose()
    }
}

/// The type a setting must have
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Expected {
    /// A string
    String,
    /// A list of strings
    Strings,
}

/// Why an extension's settings cannot be read
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SettingsError {
    /// The extension's entry in `markdown_extensions` is no table of settings
    NotATable {
        /// The extension's id
        extension: &'static str,
    },
    /// A setting of the wrong type
    WrongType {
        /// The extension's id
        extension: &'static str,
        /// The setting
        key: &'static str,
        /// The type it must have
        expected: Expected,
    },
}

impl std::fmt::Display for SettingsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotATable { extension } => write!(
                f,
                "markdown_extensions.\"{extension}\" is no table of settings; write {{}} for none"
            ),
            Self::WrongType {
                extension,
                key,
                expected,
            } => {
                let expected = match expected {
                    Expected::String => "a string",
                    Expected::Strings => "a list of strings",
                };
                write!(f, "\"{extension}\": `{key}` must be {expected}")
            }
        }
    }
}

impl std::error::Error for SettingsError {}

/// The glossary extension's settings
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GlossarySettings {
    /// The glossary file, relative to the project root
    pub file: String,
    /// Where the glossary page is served; references link to `{page}#term-{slug}`
    pub page: String,
}

/// The code link extension's settings
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CodeRefsSettings {
    /// The code index, relative to the project root
    pub index: String,
    /// The library crates indexed, by library name; empty: every library of the workspace
    pub crates: Vec<String>,
    /// The packages published on crates.io, by package name: their cards link to docs.rs
    pub on_crates_io: Vec<String>,
    /// The sources file ([`crate::sources`]), relative to the project root: the other repositories the book copies
    /// pages from; `None`: the book copies none
    pub sources: Option<String>,
}

/// The abbreviations extension's settings
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AbbreviationsSettings {
    /// The abbreviations file, relative to the project root
    pub file: String,
}

/// The review extension's settings
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReviewSettings {
    /// The review file ([`crate::review`]), relative to the project root; a missing file shows nothing
    pub file: String,
}

/// Which of the kit's extensions a book turns on, with their settings
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Settings {
    /// The glossary extension, when on
    pub glossary: Option<GlossarySettings>,
    /// The code link extension, when on
    pub code_refs: Option<CodeRefsSettings>,
    /// The abbreviations extension, when on
    pub abbreviations: Option<AbbreviationsSettings>,
    /// The review extension, when on
    pub review: Option<ReviewSettings>,
}

impl Settings {
    /// The settings from the book's extensions: `ext(id)` is the configuration of an extension the book turns on,
    /// an empty one when it takes no settings
    ///
    /// # Errors
    ///
    /// A setting has the wrong type.
    pub fn from_extensions<'a, C: ExtConfig + 'a>(
        ext: impl Fn(&str) -> Option<&'a C>,
    ) -> Result<Self, SettingsError> {
        Ok(Self {
            glossary: ext(GLOSSARY_EXTENSION)
                .map(|c| {
                    let r = Reader {
                        config: c,
                        extension: GLOSSARY_EXTENSION,
                    };
                    Ok(GlossarySettings {
                        file: r.string("file", DEFAULT_GLOSSARY)?,
                        page: r.string("page", DEFAULT_GLOSSARY_PAGE)?,
                    })
                })
                .transpose()?,
            code_refs: ext(CODE_REFS_EXTENSION)
                .map(|c| {
                    let r = Reader {
                        config: c,
                        extension: CODE_REFS_EXTENSION,
                    };
                    Ok(CodeRefsSettings {
                        index: r.string("index", DEFAULT_CODE_INDEX)?,
                        crates: r.strings("crates")?,
                        on_crates_io: r.strings("on_crates_io")?,
                        sources: r.optional("sources")?,
                    })
                })
                .transpose()?,
            abbreviations: ext(ABBREVIATIONS_EXTENSION)
                .map(|c| {
                    let r = Reader {
                        config: c,
                        extension: ABBREVIATIONS_EXTENSION,
                    };
                    Ok(AbbreviationsSettings {
                        file: r.string("file", DEFAULT_ABBREVIATIONS)?,
                    })
                })
                .transpose()?,
            review: ext(REVIEW_EXTENSION)
                .map(|c| {
                    let r = Reader {
                        config: c,
                        extension: REVIEW_EXTENSION,
                    };
                    Ok(ReviewSettings {
                        file: r.string("file", DEFAULT_REVIEW)?,
                    })
                })
                .transpose()?,
        })
    }

    /// The settings from a `markdown_extensions` table of `zensical.toml`
    ///
    /// # Errors
    ///
    /// An extension's entry is no table, or a setting has the wrong type.
    pub fn from_toml(markdown_extensions: &toml::Table) -> Result<Self, SettingsError> {
        for extension in [
            GLOSSARY_EXTENSION,
            CODE_REFS_EXTENSION,
            ABBREVIATIONS_EXTENSION,
            REVIEW_EXTENSION,
        ] {
            if markdown_extensions
                .get(extension)
                .is_some_and(|v| !v.is_table())
            {
                return Err(SettingsError::NotATable { extension });
            }
        }
        Self::from_extensions(|id| markdown_extensions.get(id).and_then(toml::Value::as_table))
    }
}

/// One extension's settings with the extension's id for errors
struct Reader<'a, C> {
    config: &'a C,
    extension: &'static str,
}

impl<C: ExtConfig> Reader<'_, C> {
    fn string(&self, key: &'static str, default: &str) -> Result<String, SettingsError> {
        self.config
            .string(key)
            .map(|v| v.unwrap_or_else(|| default.to_owned()))
            .map_err(|WrongType| self.wrong(key, Expected::String))
    }

    fn optional(&self, key: &'static str) -> Result<Option<String>, SettingsError> {
        self.config
            .string(key)
            .map_err(|WrongType| self.wrong(key, Expected::String))
    }

    fn strings(&self, key: &'static str) -> Result<Vec<String>, SettingsError> {
        self.config
            .strings(key)
            .map(Option::unwrap_or_default)
            .map_err(|WrongType| self.wrong(key, Expected::Strings))
    }

    fn wrong(&self, key: &'static str, expected: Expected) -> SettingsError {
        SettingsError::WrongType {
            extension: self.extension,
            key,
            expected,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(markdown_extensions: &str) -> Result<Settings, SettingsError> {
        Settings::from_toml(&markdown_extensions.parse::<toml::Table>().unwrap())
    }

    #[test]
    fn no_extension_on_gives_no_settings() {
        assert_eq!(settings("abbr = {}\n").unwrap(), Settings::default());
    }

    #[test]
    fn defaults_when_the_extensions_take_no_settings() {
        let s = settings(
            "\"vhrd_zensical.glossary\" = {}\n\"vhrd_zensical.code_refs\" = {}\n\"vhrd_zensical.abbreviations\" = {}\n\"vhrd_zensical.review\" = {}\n",
        )
        .unwrap();
        assert_eq!(
            s.glossary,
            Some(GlossarySettings {
                file: "docs/glossary.toml".to_owned(),
                page: "/glossary/".to_owned(),
            })
        );
        assert_eq!(
            s.code_refs,
            Some(CodeRefsSettings {
                index: "target/vz/code_index.json".to_owned(),
                crates: Vec::new(),
                on_crates_io: Vec::new(),
                sources: None,
            })
        );
        assert_eq!(
            s.abbreviations,
            Some(AbbreviationsSettings {
                file: "docs/abbreviations.toml".to_owned(),
            })
        );
        assert_eq!(
            s.review,
            Some(ReviewSettings {
                file: "target/vz/review.json".to_owned(),
            })
        );
    }

    #[test]
    fn every_setting_as_written() {
        let s = settings(
            "\"vhrd_zensical.glossary\" = { file = \"g.toml\", page = \"/terms/\" }\n\
             \"vhrd_zensical.code_refs\" = { index = \"x.json\", crates = [\"a\", \"b\"], on_crates_io = [\"a-pkg\"], sources = \"s.toml\", card_lines_max = 9 }\n\
             \"vhrd_zensical.abbreviations\" = { file = \"a.toml\" }\n\
             \"vhrd_zensical.review\" = { file = \"r.json\" }\n",
        )
        .unwrap();
        let g = s.glossary.unwrap();
        assert_eq!((g.file.as_str(), g.page.as_str()), ("g.toml", "/terms/"));
        let c = s.code_refs.unwrap();
        assert_eq!(c.index, "x.json");
        assert_eq!(c.crates, ["a", "b"]);
        assert_eq!(c.on_crates_io, ["a-pkg"]);
        assert_eq!(c.sources.as_deref(), Some("s.toml"));
        assert_eq!(s.abbreviations.unwrap().file, "a.toml");
        assert_eq!(s.review.unwrap().file, "r.json");
    }

    #[test]
    fn a_setting_of_the_wrong_type_is_an_error() {
        for (text, extension, key, expected) in [
            (
                "\"vhrd_zensical.glossary\" = { file = 1 }",
                GLOSSARY_EXTENSION,
                "file",
                Expected::String,
            ),
            (
                "\"vhrd_zensical.glossary\" = { page = [] }",
                GLOSSARY_EXTENSION,
                "page",
                Expected::String,
            ),
            (
                "\"vhrd_zensical.code_refs\" = { index = true }",
                CODE_REFS_EXTENSION,
                "index",
                Expected::String,
            ),
            (
                "\"vhrd_zensical.code_refs\" = { crates = \"a\" }",
                CODE_REFS_EXTENSION,
                "crates",
                Expected::Strings,
            ),
            (
                "\"vhrd_zensical.code_refs\" = { on_crates_io = [\"a\", 1] }",
                CODE_REFS_EXTENSION,
                "on_crates_io",
                Expected::Strings,
            ),
            (
                "\"vhrd_zensical.code_refs\" = { sources = [] }",
                CODE_REFS_EXTENSION,
                "sources",
                Expected::String,
            ),
            (
                "\"vhrd_zensical.abbreviations\" = { file = {} }",
                ABBREVIATIONS_EXTENSION,
                "file",
                Expected::String,
            ),
            (
                "\"vhrd_zensical.review\" = { file = 1 }",
                REVIEW_EXTENSION,
                "file",
                Expected::String,
            ),
        ] {
            assert_eq!(
                settings(text),
                Err(SettingsError::WrongType {
                    extension,
                    key,
                    expected
                }),
                "{text}"
            );
        }
        assert_eq!(
            settings("\"vhrd_zensical.code_refs\" = { crates = \"a\" }")
                .unwrap_err()
                .to_string(),
            "\"vhrd_zensical.code_refs\": `crates` must be a list of strings"
        );
    }

    #[test]
    fn an_entry_that_is_no_table_is_an_error() {
        let e = settings("\"vhrd_zensical.abbreviations\" = true").unwrap_err();
        assert_eq!(
            e,
            SettingsError::NotATable {
                extension: ABBREVIATIONS_EXTENSION
            }
        );
        assert!(e.to_string().contains("write {} for none"), "{e}");
    }

    /// A caller's own configuration tree, as `spl` brings one
    struct Pairs(Vec<(&'static str, Vec<&'static str>)>);

    impl ExtConfig for Pairs {
        fn string(&self, key: &str) -> Result<Option<String>, WrongType> {
            match self.0.iter().find(|(k, _)| *k == key) {
                None => Ok(None),
                Some((_, v)) if v.len() == 1 => Ok(v.first().map(|s| (*s).to_owned())),
                Some(_) => Err(WrongType),
            }
        }

        fn strings(&self, key: &str) -> Result<Option<Vec<String>>, WrongType> {
            Ok(self
                .0
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| v.iter().map(|s| (*s).to_owned()).collect()))
        }
    }

    #[test]
    fn reads_through_any_ext_config() {
        let glossary = Pairs(vec![("file", vec!["terms.toml"])]);
        let s = Settings::from_extensions(|id| (id == GLOSSARY_EXTENSION).then_some(&glossary))
            .unwrap();
        assert_eq!(s.glossary.unwrap().file, "terms.toml");
        assert_eq!(s.code_refs, None);
        let bad = Pairs(vec![("page", vec!["a", "b"])]);
        let e =
            Settings::from_extensions(|id| (id == GLOSSARY_EXTENSION).then_some(&bad)).unwrap_err();
        assert!(matches!(e, SettingsError::WrongType { key: "page", .. }));
    }
}
