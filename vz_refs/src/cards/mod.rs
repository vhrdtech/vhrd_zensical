//! The cards references show: their ids, their frame and parts, and the Markdown they hold.
//!
//! A card's contents are escaped here or rendered by the caller's [`Render`]: the Markdown extensions render with
//! Python-Markdown and Pygments, so a card's code and docs look as the site's own. Everything else of a card is
//! assembled here.

/// The classes of a card's parts, which the stylesheet `vz-refs.css` styles
pub const PART_CLASSES: [&str; 6] = [HEAD, SRC, DOC, WHERE, LINKS, MORE];

/// A card's first line: what it names, linked to its lines
pub(crate) const HEAD: &str = "vz-card-head";
/// A card's code
pub(crate) const SRC: &str = "vz-card-src";
/// A card's docs
pub(crate) const DOC: &str = "vz-card-doc";
/// Where a card's lines are
pub(crate) const WHERE: &str = "vz-card-where";
/// A card's links
pub(crate) const LINKS: &str = "vz-card-links";
/// What a card leaves out
pub(crate) const MORE: &str = "vz-card-more";

/// Characters of a card's slug at most, before its digest
const SLUG_CHARS_MAX: usize = 48;

/// Markdown as HTML, the way the site renders it
pub trait Render {
    /// Why a text could not be rendered
    type Error;

    /// `text` as safe HTML: raw HTML as text, links only to `http` and `https`, code blocks highlighted
    ///
    /// # Errors
    ///
    /// The renderer fails.
    fn markdown(&self, text: &str) -> Result<String, Self::Error>;

    /// One paragraph of Markdown as safe HTML, without its paragraph
    ///
    /// # Errors
    ///
    /// The renderer fails.
    fn inline(&self, text: &str) -> Result<String, Self::Error> {
        let out = self.markdown(text)?;
        Ok(
            match out.strip_prefix("<p>").and_then(|o| o.strip_suffix("</p>")) {
                Some(inner) if out.matches("<p>").count() == 1 => inner.to_owned(),
                _ => out,
            },
        )
    }

    /// Code highlighted as the site highlights a fenced block in `lang`
    ///
    /// # Errors
    ///
    /// The renderer fails.
    fn code(&self, text: &str, lang: &str) -> Result<String, Self::Error> {
        let longest = longest_run(text, '`');
        let fence = "`".repeat(3.max(longest + 1));
        self.markdown(&format!("{fence}{lang}\n{text}\n{fence}"))
    }
}

/// The longest run of `c` in `text`
fn longest_run(text: &str, c: char) -> usize {
    let (mut longest, mut run) = (0, 0);
    for t in text.chars() {
        run = if t == c { run + 1 } else { 0 };
        longest = longest.max(run);
    }
    longest
}

/// The id of the card for `key`, unique per key: `vz-card-` with a slug of the key and its digest
pub fn card_id(key: &str) -> String {
    let digest = sha1_smol::Sha1::from(key.as_bytes()).digest().to_string();
    let slug: String = slug(key).chars().take(SLUG_CHARS_MAX).collect();
    format!("vz-card-{slug}-{}", digest.get(..6).unwrap_or(&digest))
}

/// `text` lowercased, every run of characters other than `a-z` and `0-9` one `-`, none at either end
pub(crate) fn slug(text: &str) -> String {
    let mut out = String::new();
    let mut dash = false;
    for c in text.to_lowercase().chars() {
        if c.is_ascii_lowercase() || c.is_ascii_digit() {
            if dash && !out.is_empty() {
                out.push('-');
            }
            dash = false;
            out.push(c);
        } else {
            dash = true;
        }
    }
    out
}

/// `text` with `&`, `<`, `>`, `"` and `'` escaped, as Python's `html.escape` writes them
pub fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#x27;"),
            c => out.push(c),
        }
    }
    out
}

/// A card of `kind` around its contents, which are escaped or rendered already
pub(crate) fn frame(card_id: &str, kind: &str, body: &str) -> String {
    format!(r#"<div class="vz-card vz-card-{kind}" id="{card_id}" role="tooltip">{body}</div>"#)
}

/// One part of a card
pub(crate) fn part(class: &str, inner: &str) -> String {
    format!(r#"<div class="{class}">{inner}</div>"#)
}

/// A card's first line, a link to what it names on the forge
pub(crate) fn head(text: &str, href: &str) -> String {
    part(
        HEAD,
        &format!(r#"<a href="{}">{}</a>"#, escape(href), escape(text)),
    )
}

/// A card's links, text and address each
pub(crate) fn links(links: &[(&str, &str)]) -> String {
    let links: Vec<String> = links
        .iter()
        .map(|(t, u)| format!(r#"<a href="{}">{}</a>"#, escape(u), escape(t)))
        .collect();
    part(LINKS, &links.join(" · "))
}

/// The note that `n` lines are left out, or nothing
pub(crate) fn more_lines(n: usize) -> String {
    if n == 0 {
        return String::new();
    }
    let s = if n == 1 { "" } else { "s" };
    part(MORE, &format!("{n} more line{s} in the source"))
}

/// At most `lines_max` lines of `text`, and how many were left out
pub(crate) fn first_lines(text: &str, lines_max: usize) -> (String, usize) {
    let lines: Vec<&str> = text.split('\n').collect();
    let kept = lines.len().min(lines_max);
    (lines[..kept].join("\n"), lines.len() - kept)
}

#[cfg(test)]
pub(crate) mod tests;
