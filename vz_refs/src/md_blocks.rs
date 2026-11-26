//! The top-level blocks of a page's Markdown, as line spans, in the order Python-Markdown renders them.
//!
//! A close reading, not a parser: it knows the blocks the kit's books write (paragraphs, headings, lists, tables,
//! fences, admonitions, tabs, definition lists, raw HTML). A caller checks that it found as many blocks as the page
//! rendered, and falls back when not.

/// The first and last line, 1-based, of each top-level block that renders to an element of its own
///
/// Front matter, link and abbreviation definitions, footnotes and lone attribute lists render none and are left out.
pub fn blocks(source: &str) -> Vec<(usize, usize)> {
    blocks_with(source, |_| None)
}

/// [`blocks`], with the blocks of included files
///
/// A snippet line (`--8<-- "path"`) alone at the top level stands for as many blocks as `included(path)` says, each
/// spanning that line; `None` (a file it cannot read) counts it as one.
pub fn blocks_with(source: &str, included: impl Fn(&str) -> Option<usize>) -> Vec<(usize, usize)> {
    let lines: Vec<&str> = source.split('\n').collect();
    let mut out: Vec<(usize, usize, Kind)> = Vec::new();
    let mut i = front_matter(&lines);
    while i < lines.len() {
        let line = lines[i];
        if line.trim().is_empty() {
            i += 1;
            continue;
        }
        let (end, kind) = block(&lines, i);
        // tabs in a row are one set; definitions in a row one list; a definition joins its term
        let merge = matches!(
            (out.last(), kind),
            (Some(&(_, _, Kind::Tabs)), Kind::Tabs)
                | (Some(&(_, _, Kind::Defs | Kind::Para)), Kind::Defs)
        );
        if let Some(k) = (end == i + 1)
            .then(|| snippet(line))
            .flatten()
            .and_then(&included)
        {
            out.extend(std::iter::repeat_n((i + 1, i + 1, Kind::Other), k));
        } else if kind == Kind::None {
        } else if merge && let Some(last) = out.last_mut() {
            last.1 = end;
            last.2 = kind;
        } else {
            out.push((i + 1, end, kind));
        }
        i = end;
    }
    out.into_iter().map(|(a, b, _)| (a, b)).collect()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    /// Renders no element
    None,
    Para,
    Other,
    Tabs,
    Defs,
}

/// The line after a front matter block at the top, else 0
fn front_matter(lines: &[&str]) -> usize {
    if lines.first().map(|l| l.trim_end()) != Some("---") {
        return 0;
    }
    lines
        .iter()
        .skip(1)
        .position(|l| matches!(l.trim_end(), "---" | "..."))
        .map_or(0, |p| p + 2)
}

/// The path a snippet line includes: `--8<-- "path"`
fn snippet(line: &str) -> Option<&str> {
    let rest = line.strip_prefix("--8<--")?.trim();
    rest.strip_prefix('"')?
        .strip_suffix('"')
        .or_else(|| rest.strip_prefix('\'')?.strip_suffix('\''))
}

fn indent(line: &str) -> usize {
    line.len() - line.trim_start_matches(' ').len()
}

fn blank(line: &str) -> bool {
    line.trim().is_empty()
}

/// The fence a line opens: its character and length
fn fence(line: &str) -> Option<(char, usize)> {
    if indent(line) > 3 {
        return None;
    }
    let t = line.trim_start();
    let c = t.chars().next().filter(|c| matches!(c, '`' | '~'))?;
    let n = t.chars().take_while(|&x| x == c).count();
    (n >= 3).then_some((c, n))
}

fn list_item(line: &str) -> bool {
    let t = line.trim_start();
    if indent(line) > 3 {
        return false;
    }
    if let Some(rest) = t.strip_prefix(['-', '*', '+']) {
        return rest.starts_with(' ') && !is_rule(line);
    }
    let digits = t.chars().take_while(char::is_ascii_digit).count();
    digits > 0 && t[digits..].starts_with(". ")
}

fn is_rule(line: &str) -> bool {
    let t: String = line.chars().filter(|c| !c.is_whitespace()).collect();
    t.len() >= 3
        && (t.chars().all(|c| c == '-')
            || t.chars().all(|c| c == '*')
            || t.chars().all(|c| c == '_'))
}

fn heading(line: &str) -> bool {
    let t = line.trim_start();
    indent(line) <= 3 && {
        let n = t.chars().take_while(|&c| c == '#').count();
        (1..=6).contains(&n) && t[n..].chars().next().is_none_or(char::is_whitespace)
    }
}

/// A line that only defines: a link reference, an abbreviation, a footnote, or attributes for the block before
fn definition(line: &str) -> bool {
    let t = line.trim();
    (t.starts_with("*[") && t.contains("]:"))
        || (t.starts_with("[^") && t.contains("]:"))
        || (t.starts_with('[') && t.contains("]:") && indent(line) <= 3)
        || (t.starts_with("{:") && t.ends_with('}'))
}

/// The raw HTML block starting here: the tag it opens, or `!--` for a comment
fn html_open(line: &str) -> Option<String> {
    let t = line.trim_start();
    if indent(line) > 3 {
        return None;
    }
    if t.starts_with("<!--") {
        return Some("!--".into());
    }
    let rest = t.strip_prefix('<')?;
    let name: String = rest
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric())
        .collect();
    BLOCK_TAGS
        .contains(&name.to_ascii_lowercase().as_str())
        .then_some(name)
}

/// Python-Markdown's block-level tags (`markdown.util.BLOCK_LEVEL_ELEMENTS`): a line starting with another tag, such as
/// `<span>`, starts a paragraph
const BLOCK_TAGS: [&str; 59] = [
    "address",
    "article",
    "aside",
    "blockquote",
    "body",
    "canvas",
    "center",
    "colgroup",
    "dd",
    "details",
    "div",
    "dl",
    "dt",
    "fieldset",
    "figcaption",
    "figure",
    "footer",
    "form",
    "group",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "header",
    "hgroup",
    "hr",
    "html",
    "iframe",
    "legend",
    "li",
    "main",
    "map",
    "math",
    "menu",
    "nav",
    "noscript",
    "object",
    "ol",
    "option",
    "output",
    "p",
    "pre",
    "progress",
    "script",
    "section",
    "style",
    "summary",
    "table",
    "tbody",
    "td",
    "textarea",
    "tfoot",
    "th",
    "thead",
    "tr",
    "ul",
    "video",
];

/// The line after a block starting at `i`, and its kind
fn block(lines: &[&str], i: usize) -> (usize, Kind) {
    let line = lines[i];
    let t = line.trim_start();
    if let Some((c, len)) = fence(line) {
        let close = (i + 1..lines.len()).find(|&k| {
            let x = lines[k].trim();
            x.chars().all(|ch| ch == c) && x.len() >= len
        });
        return (close.map_or(lines.len(), |k| k + 1), Kind::Other);
    }
    if let Some(tag) = html_open(line) {
        return (html_end(lines, i, &tag), Kind::Other);
    }
    if heading(line) || is_rule(line) {
        return (i + 1, Kind::Other);
    }
    let body = indented_body_from(lines, i + 1);
    if t.starts_with("!!!") || t.starts_with("???") || indent(line) >= 4 {
        return (body, Kind::Other);
    }
    if t.starts_with("===") {
        return (body, Kind::Tabs);
    }
    if t.starts_with(": ") {
        return (body, Kind::Defs);
    }
    if list_item(line) {
        return (list_end(lines, i), Kind::Other);
    }
    paragraph(lines, i)
}

/// The line after a raw HTML block opening `tag` at `i` (`!--`: a comment), where its tag closes
fn html_end(lines: &[&str], i: usize, tag: &str) -> usize {
    if tag == "!--" {
        let close = (i..lines.len()).find(|&k| lines[k].contains("-->"));
        return close.map_or(lines.len(), |k| k + 1);
    }
    let (open, shut) = (format!("<{tag}"), format!("</{tag}"));
    let mut depth = 0usize;
    for (k, l) in lines.iter().enumerate().skip(i) {
        depth += l.matches(&open).count();
        depth = depth.saturating_sub(l.matches(&shut).count());
        if depth == 0 {
            return k + 1;
        }
    }
    lines.len()
}

/// The line after a list starting at `i`: its items and their indented lines, blank lines between them, until a line
/// of text that is neither
fn list_end(lines: &[&str], i: usize) -> usize {
    let mut end = i + 1;
    let mut j = i + 1;
    while j < lines.len() {
        let l = lines[j];
        if blank(l) {
            j += 1;
            continue;
        }
        // an item, an indented line, or a lazy continuation of the item's paragraph
        let lazy = !blank(lines[j - 1]) && !heading(l) && fence(l).is_none();
        if indent(l) >= 2 || list_item(l) || lazy {
            j += 1;
            end = j;
        } else {
            break;
        }
    }
    end
}

/// A paragraph, a table or definitions: lines of text up to a blank line or a line that starts another block
fn paragraph(lines: &[&str], i: usize) -> (usize, Kind) {
    let mut end = i + 1;
    while end < lines.len() {
        let l = lines[end];
        if blank(l) || heading(l) || fence(l).is_some() {
            break;
        }
        end += 1;
    }
    if lines[i..end].iter().all(|l| definition(l)) {
        // a footnote takes its indented paragraphs after it
        let footnote = lines[i].trim().starts_with("[^");
        return (
            if footnote {
                indented_body_from(lines, end)
            } else {
                end
            },
            Kind::None,
        );
    }
    if lines[i + 1..end].iter().any(|l| l.starts_with(": ")) {
        return (indented_body_from(lines, end), Kind::Defs);
    }
    (end, Kind::Para)
}

/// The line after `end` and the lines indented 4 or more that follow it, blank lines between them
fn indented_body_from(lines: &[&str], end: usize) -> usize {
    let mut out = end;
    let mut j = end;
    while j < lines.len() {
        if blank(lines[j]) {
            j += 1;
        } else if indent(lines[j]) >= 4 {
            j += 1;
            out = j;
        } else {
            break;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paragraphs_headings_and_lists() {
        let page = "---\nlevel: basic\n---\n\n# Title\n\nOne\ntwo\n\n- a\n  more\n\n- b\n\n## Next\n\nThree\n";
        assert_eq!(blocks(page), [(5, 5), (7, 8), (10, 13), (15, 15), (17, 17)]);
    }

    #[test]
    fn fences_html_and_admonitions_are_one_block_each() {
        let page = "<!-- a\nnote -->\n<div class=\"x\">\n<div>in</div>\n</div>\n\n```rust\n\nfn f() {}\n```\n\n!!! note\n\n    Body\n\n    More\n\nAfter\n";
        assert_eq!(blocks(page), [(1, 2), (3, 5), (7, 10), (12, 16), (18, 18)]);
    }

    #[test]
    fn tabs_in_a_row_are_one_set_and_definitions_join_their_term() {
        let page =
            "=== \"A\"\n\n    a\n\n=== \"B\"\n\n    b\n\nTerm\n: what it is\n\nOther\n:   more\n";
        assert_eq!(blocks(page), [(1, 7), (9, 13)]);
    }

    #[test]
    fn a_snippet_line_stands_for_the_blocks_it_includes() {
        let page = "One\n\n--8<-- \"plots/a.txt\"\n\nTwo\n";
        let blocks = blocks_with(page, |p| (p == "plots/a.txt").then_some(2));
        assert_eq!(blocks, [(1, 1), (3, 3), (3, 3), (5, 5)]);
        assert_eq!(blocks_with(page, |_| None).len(), 3, "unread: one block");
    }

    #[test]
    fn definitions_render_nothing() {
        let page = "Text[^1].\n\n[^1]: A note\n\n    more of it\n\n*[HTML]: markup\n[link]: https://x\n\nEnd\n";
        assert_eq!(blocks(page), [(1, 1), (10, 10)]);
    }
}
