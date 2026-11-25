//! An item's docs as its card shows them: rustdoc's intra-doc links resolved, bounded in length.

use super::Part;
use std::collections::BTreeMap;
use vz_core::code_index::{Entry, Kind};

/// The languages of a fence rustdoc compiles as Rust, whose `# ` lines it hides
const RUSTDOC_LANGS: [&str; 9] = [
    "",
    "rust",
    "no_run",
    "ignore",
    "should_panic",
    "compile_fail",
    "edition2018",
    "edition2021",
    "edition2024",
];

/// The fence a stripped line opens: a run of three backticks or more, or of three tildes or more
fn fence_of(stripped: &str) -> Option<&str> {
    let c = stripped.chars().next().filter(|c| matches!(c, '`' | '~'))?;
    let run = stripped.len() - stripped.trim_start_matches(c).len();
    (run >= 3).then(|| &stripped[..run])
}

/// Whether `c` may stand in a fence's language list
fn is_lang_char(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '_' | ',' | '-')
}

/// `fix` applied to the parts of a doc outside fenced code; rustdoc's hidden lines dropped inside Rust fences
fn outside_fences(text: &str, mut fix: impl FnMut(&str) -> String) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut prose: Vec<&str> = Vec::new();
    let mut fence: Option<&str> = None;
    let mut hide = false;
    for line in text.split('\n') {
        let stripped = line.trim();
        match fence {
            None => {
                if let Some(f) = fence_of(stripped) {
                    out.push(fix(&prose.join("\n")));
                    prose.clear();
                    let rest = stripped[f.len()..].trim_start();
                    let lang = &rest[..rest.find(|c| !is_lang_char(c)).unwrap_or(rest.len())];
                    hide = lang.split(',').all(|l| RUSTDOC_LANGS.contains(&l));
                    out.push(format!("{f}{}", if hide { "rust" } else { lang }));
                    fence = Some(f);
                } else {
                    prose.push(line);
                }
            }
            Some(f) if stripped.starts_with(f) => {
                out.push(f.to_owned());
                fence = None;
            }
            Some(_) => {
                if !(hide && (stripped == "#" || stripped.starts_with("# "))) {
                    out.push(line.to_owned());
                }
            }
        }
    }
    out.push(fix(&prose.join("\n")));
    out.join("\n")
}

/// The path a link target names, as rustdoc writes one: an optional `kind@`, a path, an optional `()` or `!`
fn target_path(target: &str) -> Option<&str> {
    let path = match target.split_once('@') {
        Some((kind, path)) if !kind.is_empty() && kind.chars().all(|c| c.is_ascii_lowercase()) => {
            path
        }
        Some(_) => return None,
        None => target,
    };
    let path = path
        .strip_suffix("()")
        .or_else(|| path.strip_suffix('!'))
        .unwrap_or(path);
    let ident = |s: &str| {
        let mut cs = s.chars();
        cs.next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
            && cs.all(|c| c.is_ascii_alphanumeric() || c == '_')
    };
    path.split("::").all(ident).then_some(path)
}

/// The end of `[text]` opening at `i`: the index of its `]`, with no bracket inside and some text
fn bracket(chars: &[char], i: usize) -> Option<usize> {
    if chars.get(i) != Some(&'[') {
        return None;
    }
    let j = i
        + 1
        + chars
            .get(i + 1..)?
            .iter()
            .position(|c| matches!(c, '[' | ']'))?;
    (chars[j] == ']' && j > i + 1).then_some(j)
}

fn text(chars: &[char]) -> String {
    chars.iter().collect()
}

/// `text` with each link that `at` opens at an index replaced by `replace(at)`: `Some((end, with))` replaces up to
/// `end`, `None` keeps the `[`
fn replace_links(
    text: &str,
    mut replace: impl FnMut(&[char], usize) -> Option<(usize, String)>,
) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '['
            && let Some((end, with)) = replace(&chars, i)
        {
            out.push_str(&with);
            i = end;
            continue;
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// The reference definitions of `text` taken out into `defs`: `[name]: target` lines
fn strip_defs(text: &str, defs: &mut BTreeMap<String, String>) -> String {
    let mut lines = Vec::new();
    for line in text.split('\n') {
        if let Some((name, target)) = definition(line) {
            defs.insert(name.trim_matches('`').to_owned(), target.to_owned());
        } else {
            lines.push(line);
        }
    }
    lines.join("\n")
}

/// The name and target of a reference definition line
fn definition(line: &str) -> Option<(&str, &str)> {
    let rest = line.trim_start().strip_prefix('[')?;
    let close = rest.find(']')?;
    let (name, rest) = (&rest[..close], &rest[close + 1..]);
    let target = rest.strip_prefix(':')?.trim();
    (!name.is_empty() && !target.is_empty() && !target.contains(char::is_whitespace))
        .then_some((name, target))
}

/// The three link forms of prose, each replaced by `link(text, target)`; `defs` holds the reference definitions
fn fix_links(
    prose: &str,
    defs: &BTreeMap<String, String>,
    link: &dyn Fn(&str, &str) -> String,
) -> String {
    let prose = replace_links(prose, |chars, i| inline_link(chars, i, link));
    let prose = replace_links(&prose, |chars, i| reference_link(chars, i, defs, link));
    replace_links(&prose, |chars, i| shortcut_link(chars, i, defs, link))
}

/// `[text](target)` at `i`
fn inline_link(
    chars: &[char],
    i: usize,
    link: &dyn Fn(&str, &str) -> String,
) -> Option<(usize, String)> {
    let j = bracket(chars, i)?;
    if chars.get(j + 1) != Some(&'(') {
        return None;
    }
    let k = j
        + 2
        + chars
            .get(j + 2..)?
            .iter()
            .position(|c| matches!(c, '(' | ')') || c.is_whitespace())?;
    (chars[k] == ')' && k > j + 2).then(|| {
        (
            k + 1,
            link(&text(&chars[i + 1..j]), &text(&chars[j + 2..k])),
        )
    })
}

/// `[text][name]` or `[text][]` at `i`
fn reference_link(
    chars: &[char],
    i: usize,
    defs: &BTreeMap<String, String>,
    link: &dyn Fn(&str, &str) -> String,
) -> Option<(usize, String)> {
    let j = bracket(chars, i)?;
    if chars.get(j + 1) != Some(&'[') {
        return None;
    }
    let k = j
        + 2
        + chars
            .get(j + 2..)?
            .iter()
            .position(|c| matches!(c, '[' | ']'))?;
    if chars[k] != ']' {
        return None;
    }
    let shown = text(&chars[i + 1..j]);
    let name = text(&chars[j + 2..k]);
    let (key, fallback) = if name.is_empty() {
        (shown.trim_matches('`').to_owned(), shown.clone())
    } else {
        (name.clone(), name)
    };
    let target = defs.get(&key).cloned().unwrap_or(fallback);
    Some((k + 1, link(&shown, &target)))
}

/// `[text]` at `i`, not followed by a target: a link when it names a path or a definition, else kept
fn shortcut_link(
    chars: &[char],
    i: usize,
    defs: &BTreeMap<String, String>,
    link: &dyn Fn(&str, &str) -> String,
) -> Option<(usize, String)> {
    let j = bracket(chars, i)?;
    if chars
        .get(j + 1)
        .is_some_and(|c| matches!(c, '(' | '[' | ':'))
    {
        return None;
    }
    let shown = text(&chars[i + 1..j]);
    let key = shown.trim_matches('`');
    let with = if target_path(key).is_some() || defs.contains_key(key) {
        link(&shown, defs.get(key).map_or(shown.as_str(), String::as_str))
    } else {
        format!("[{shown}]")
    };
    Some((j + 1, with))
}

impl Part {
    /// The address an intra-doc link in `at`'s docs opens: the target when it is one, else the named item's lines
    fn doc_target(&self, target: &str, at: &Entry) -> Option<String> {
        if target.starts_with("https://") || target.starts_with("http://") {
            return Some(target.to_owned());
        }
        let name = target_path(target.trim_matches('`'))?;
        let name = match name.strip_prefix("Self::") {
            Some(rest) => {
                let parent = match at.kind {
                    Kind::Fn | Kind::Const | Kind::Field | Kind::Variant => {
                        at.path.rsplit_once("::").map_or("", |(p, _)| p)
                    }
                    _ => at.path.as_str(),
                };
                format!("{parent}::{rest}")
            }
            None => name.to_owned(),
        };
        let name = ["crate::", "self::", "super::"]
            .iter()
            .find_map(|p| name.strip_prefix(p))
            .unwrap_or(&name);
        self.near(name, at).map(|i| i.url.clone())
    }

    /// rustdoc's intra-doc links in `at`'s docs as Markdown links to the items' lines, or plain text
    pub(crate) fn doc_links(&self, doc: &str, at: &Entry) -> String {
        let link = |text: &str, target: &str| match self.doc_target(target, at) {
            Some(u) => format!("[{text}]({u})"),
            None => text.to_owned(),
        };
        let mut defs = BTreeMap::new();
        outside_fences(doc, |prose| {
            let prose = strip_defs(prose, &mut defs);
            fix_links(&prose, &defs, &link)
        })
    }
}

/// The blocks of `doc` that fit in `chars_max` characters, a first paragraph too long cut after the last sentence
/// that fits (its first sentence always); and whether any was left out
pub fn bounded(doc: &str, chars_max: usize) -> (String, bool) {
    let mut blocks: Vec<String> = Vec::new();
    let mut current: Vec<&str> = Vec::new();
    let mut fence: Option<&str> = None;
    for line in doc.split('\n') {
        let stripped = line.trim();
        match fence {
            None => fence = fence_of(stripped),
            Some(f) if stripped.starts_with(f) => fence = None,
            Some(_) => {}
        }
        if fence.is_none() && stripped.is_empty() && !current.is_empty() {
            blocks.push(current.join("\n"));
            current.clear();
        } else if !stripped.is_empty() || fence.is_some() {
            current.push(line);
        }
    }
    if !current.is_empty() {
        blocks.push(current.join("\n"));
    }
    let mut kept: Vec<String> = Vec::new();
    let mut size = 0;
    for b in &blocks {
        let len = b.chars().count();
        if size + len <= chars_max {
            kept.push(b.clone());
            size += len;
            continue;
        }
        if kept.is_empty() {
            kept.push(if fence_of(b.trim_start()).is_some() {
                b.clone()
            } else {
                sentences(b, chars_max)
            });
        }
        break;
    }
    let out = kept.join("\n\n");
    let cut = out != blocks.join("\n\n");
    (out, cut)
}

/// The sentences of `paragraph` that fit in `chars_max` characters, the first always; a code span is never cut
fn sentences(paragraph: &str, chars_max: usize) -> String {
    let chars: Vec<char> = paragraph.chars().collect();
    let mut ends = Vec::new();
    let mut ticks = 0;
    for (p, &c) in chars.iter().enumerate() {
        if matches!(c, '.' | '!' | '?') && ticks % 2 == 0 {
            let after = &chars[p + 1..];
            let space = after.iter().take_while(|c| c.is_whitespace()).count();
            if space > 0
                && after.get(space).is_some_and(|c| {
                    c.is_ascii_uppercase() || matches!(c, '`' | '[' | '(' | '*' | '_')
                })
            {
                ends.push(p + 1);
            }
        }
        if c == '`' {
            ticks += 1;
        }
    }
    let end = ends
        .iter()
        .rev()
        .find(|&&e| e <= chars_max)
        .or(ends.first())
        .copied()
        .unwrap_or(chars.len());
    text(&chars[..end])
}
