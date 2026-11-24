//! The footer build stamp's markers and default paths, which `vz stamp` writes and `spl`'s `book-build-stamp`
//! lint looks for.
//!
//! The stamp is a partial under the theme's `custom_dir` that the theme's copyright partial includes:
//!
//! ```html
//! <div class="book-stamp" data-built="1764000000">ww_tape 0.3.0 · abc1234 · built 2025-11-24 16:00 UTC
//! (<span class="book-stamp-age">just built</span>)</div>
//! ```
//!
//! The age script, registered as `extra_javascript`, turns `data-built` into the build's age on the reader's
//! clock. A file carrying [`STAMP_MARKER`] carries the stamp; a file carrying [`AGE_MARKER`] carries the age
//! script. The partial never carries [`AGE_MARKER`], so a stamp without its script is told apart.

/// The class of the stamp's element
pub const STAMP_CLASS: &str = "book-stamp";

/// The class of the element the age script fills with the build's age
pub const AGE_CLASS: &str = "book-stamp-age";

/// The stamp element's attribute holding the build time in Unix seconds
pub const BUILT_ATTRIBUTE: &str = "data-built";

/// Text that marks a file carrying the stamp
pub const STAMP_MARKER: &str = STAMP_CLASS;

/// Text that marks the age script: in its first line, never in the partial
pub const AGE_MARKER: &str = "vz-stamp-age";

/// The theme's partial that includes the stamp, relative to the theme's `custom_dir`
pub const THEME_PARTIAL: &str = "partials/copyright.html";

/// The stamp partial when none is named, relative to the theme's `custom_dir`
pub const DEFAULT_STAMP_PARTIAL: &str = "partials/vz-stamp.html";

/// The age script when none is named, relative to the docs folder
pub const DEFAULT_AGE_SCRIPT: &str = "javascripts/vz-stamp-age.js";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_partial_cannot_pass_for_the_age_script() {
        for text in [
            STAMP_CLASS,
            AGE_CLASS,
            BUILT_ATTRIBUTE,
            DEFAULT_STAMP_PARTIAL,
        ] {
            assert!(!text.contains(AGE_MARKER), "{text}");
        }
        assert!(DEFAULT_AGE_SCRIPT.contains(AGE_MARKER));
    }
}
