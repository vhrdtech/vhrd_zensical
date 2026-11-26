//! The files a book serves beside its pages: the cards' stylesheet and script, the code colours, and the review
//! marks' stylesheet and script.
//!
//! `vz` writes them into the book's docs folder at every build; they never belong in git. The book registers
//! them in `zensical.toml`:
//!
//! ```toml
//! [project]
//! extra_css = ["stylesheets/vz-refs.css", "stylesheets/vz-code.css", "stylesheets/vz-review.css"]
//! extra_javascript = ["javascripts/vz-refs.js", "javascripts/vz-review.js"]
//! ```
//!
//! The classes the stylesheet styles are those the cards ([`crate::cards`]) and the Python extensions write: `vz-ref`, `vz-term`, `vz-code`,
//! `vz-file`, `vz-cards`, `vz-card` and its parts, `vz-gloss`; a reference names its card in `data-vz-card`.

/// One file a book serves
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Asset {
    /// Where it goes when the book names no other place, relative to the docs folder
    pub path: &'static str,
    /// What it is, for a report: `the cards' stylesheet`
    pub what: &'static str,
    /// Its text
    pub text: &'static str,
}

/// The cards' stylesheet: hidden until the script opens one, colours from the theme's variables
pub const CARD_CSS: Asset = Asset {
    path: "stylesheets/vz-refs.css",
    what: "the cards' stylesheet",
    text: include_str!("../assets/vz-refs.css"),
};

/// The script that opens a reference's card beside it on hover, keyboard focus and a first tap
pub const CARD_SCRIPT: Asset = Asset {
    path: "javascripts/vz-refs.js",
    what: "the cards' script",
    text: include_str!("../assets/vz-refs.js"),
};

/// The code colours: GitHub's palette over Pygments' token classes, light and dark
pub const CODE_CSS: Asset = Asset {
    path: "stylesheets/vz-code.css",
    what: "the code colours",
    text: include_str!("../assets/vz-code.css"),
};

/// The proofreading badge and the review marks ([`crate::review`]): hidden marks, light and dark palettes
pub const REVIEW_CSS: Asset = Asset {
    path: "stylesheets/vz-review.css",
    what: "the review marks' stylesheet",
    text: include_str!("../assets/vz-review.css"),
};

/// The script that turns the review marks on and off: the page's button and the `r` key, kept in localStorage
pub const REVIEW_SCRIPT: Asset = Asset {
    path: "javascripts/vz-review.js",
    what: "the review marks' script",
    text: include_str!("../assets/vz-review.js"),
};

/// Every asset, in the order a report lists them
pub const ALL: [Asset; 5] = [CARD_CSS, CARD_SCRIPT, CODE_CSS, REVIEW_CSS, REVIEW_SCRIPT];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_asset_says_it_is_written_by_vz() {
        for a in ALL {
            let first = a.text.lines().next().unwrap_or_default();
            assert!(first.contains("vz:"), "{}: {first}", a.path);
            assert!(a.text.contains("(do not edit)"), "{}", a.path);
            for old in ["spl-", "spl_", "`spl", "--sl-"] {
                assert!(!a.text.contains(old), "{} names {old}", a.path);
            }
        }
    }

    #[test]
    fn the_stylesheet_styles_what_the_script_toggles() {
        let script = CARD_SCRIPT.text;
        assert!(script.contains("a.vz-ref[data-vz-card]"));
        assert!(script.contains("\"data-vz-card\""));
        assert!(script.contains("\"vz-open\""));
        assert!(script.contains(".vz-cards"));
        for class in [".vz-ref", ".vz-card.vz-open", ".vz-cards"] {
            assert!(
                CARD_CSS.text.contains(class),
                "the stylesheet lacks {class}"
            );
        }
    }

    #[test]
    fn the_stylesheet_styles_every_class_of_a_card() {
        for class in [
            "vz-ref",
            "vz-term",
            "vz-code",
            "vz-file",
            "vz-cards",
            "vz-card",
            "vz-card-head",
            "vz-card-src",
            "vz-card-doc",
            "vz-card-where",
            "vz-card-links",
            "vz-card-more",
            "vz-gloss",
        ] {
            assert!(
                CARD_CSS.text.contains(&format!(".{class}")),
                "the stylesheet lacks .{class}"
            );
        }
    }

    #[test]
    fn the_review_stylesheet_styles_what_the_script_and_extension_write() {
        let script = REVIEW_SCRIPT.text;
        for name in [
            "\"vz-review-on\"",
            "\"vz-review-ready\"",
            "button.vz-review-toggle",
            "\"aria-pressed\"",
        ] {
            assert!(script.contains(name), "the script lacks {name}");
        }
        for rule in [
            ".vz-review-on .md-typeset [data-vz-review]",
            "[data-vz-review=\"read\"]",
            "[data-vz-review=\"stale\"]",
            "[data-vz-review=\"partial\"]",
            ".vz-review-ready .md-typeset .vz-review-switch",
            ".vz-review-badge",
            ".vz-review-toggle",
            "[data-md-color-scheme=\"slate\"]",
        ] {
            assert!(
                REVIEW_CSS.text.contains(rule),
                "the stylesheet lacks {rule}"
            );
        }
    }

    #[test]
    fn the_paths_are_distinct_and_named_vz() {
        for a in ALL {
            let name = a.path.rsplit('/').next().unwrap_or_default();
            assert!(name.starts_with("vz-"), "{}", a.path);
        }
        for (i, a) in ALL.iter().enumerate() {
            assert!(ALL[..i].iter().all(|b| b.path != a.path), "{}", a.path);
        }
    }
}
