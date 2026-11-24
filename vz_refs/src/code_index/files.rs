//! The project's text files that file references name (`` [`resume.rs`] ``, `` [`crashes.rs:resume`] ``), with the
//! sections marked in them.
//!
//! The files are those git tracks, so a link opens a file that is there at the build's commit; outside a git
//! checkout the list is empty. Files that are not UTF-8 text and the directories the caller names are left out.
//!
//! A section is marked as the snippet includes mark it: a line holding `--8<-- [start:NAME]` opens it, a line holding
//! `--8<-- [end:NAME]` closes it, and the section is the lines between. Markers that do not pair up are kept as the
//! file's problem, which fails only a page that references the file.

use serde::{Deserialize, Serialize};
use sl_model::Project;
use std::collections::BTreeMap;

/// What opens and closes a marked section
const MARKER: &str = "--8<-- [";

/// One text file
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileEntry {
    /// Its path, relative to the project root
    pub path: String,
    /// How many lines it has
    pub lines: usize,
    /// Its marked sections: the first and last line between the markers, from 1
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub sections: BTreeMap<String, [usize; 2]>,
    /// Markers that do not pair up, as `file:line: what is wrong`; its sections are then left out
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub problem: Option<String>,
}

/// A marked section that does not pair up
#[derive(Clone, Debug, PartialEq, Eq)]
struct SectionError {
    file: String,
    line: usize,
    problem: String,
}

impl std::fmt::Display for SectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}: {}", self.file, self.line, self.problem)
    }
}

/// Every text file git tracks under the project's root, sorted by path, leaving out the directories `skip`
/// (relative to the root)
pub(crate) fn files(project: &Project, skip: &[&str]) -> Vec<FileEntry> {
    let Ok(tracked) = project.tracked() else {
        return Vec::new();
    };
    let mut paths: Vec<&String> = tracked
        .iter()
        .filter(|p| {
            !skip
                .iter()
                .any(|d| p.strip_prefix(d).is_some_and(|r| r.starts_with('/')))
        })
        .collect();
    paths.sort();
    paths
        .into_iter()
        .filter_map(|path| {
            let text = read(&project.root.join(path))?;
            let (sections, problem) = match sections(path, &text) {
                Ok(s) => (s, None),
                Err(e) => (BTreeMap::new(), Some(e.to_string())),
            };
            Some(FileEntry {
                path: path.clone(),
                lines: text.lines().count(),
                sections,
                problem,
            })
        })
        .collect()
}

/// A file's text: None when it cannot be read or is not UTF-8 text
fn read(path: &std::path::Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    if bytes.contains(&0) {
        return None;
    }
    String::from_utf8(bytes).ok()
}

/// The sections marked in a file
fn sections(path: &str, text: &str) -> Result<BTreeMap<String, [usize; 2]>, SectionError> {
    let err = |line: usize, problem: String| SectionError {
        file: path.to_owned(),
        line,
        problem,
    };
    let mut open: BTreeMap<String, usize> = BTreeMap::new();
    let mut out = BTreeMap::new();
    for (i, line) in text.lines().enumerate() {
        let n = i + 1;
        let Some(at) = line.find(MARKER) else {
            continue;
        };
        let rest = line.get(at + MARKER.len()..).unwrap_or("");
        let Some(close) = rest.find(']') else {
            continue;
        };
        let Some((what, name)) = rest.get(..close).and_then(|m| m.split_once(':')) else {
            continue;
        };
        match what {
            "start" => {
                if out.contains_key(name) || open.insert(name.to_owned(), n).is_some() {
                    return Err(err(n, format!("section `{name}` is marked twice")));
                }
            }
            "end" => {
                let Some(start) = open.remove(name) else {
                    return Err(err(n, format!("section `{name}` ends without a start")));
                };
                out.insert(
                    name.to_owned(),
                    [start + 1, n.saturating_sub(1).max(start + 1)],
                );
            }
            _ => {}
        }
    }
    if let Some((name, line)) = open.into_iter().next() {
        return Err(err(line, format!("section `{name}` starts and never ends")));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sections_are_the_lines_between_their_markers() {
        let text = "a\n// --8<-- [start:one]\nb\nc\n// --8<-- [end:one]\n# --8<-- [start:two]\nd\n# --8<-- [end:two]\n";
        let s = sections("f.rs", text).unwrap();
        assert_eq!(s.get("one"), Some(&[3, 4]));
        assert_eq!(s.get("two"), Some(&[7, 7]));
    }

    #[test]
    fn markers_that_do_not_pair_are_errors() {
        let e = sections("f.rs", "// --8<-- [start:x]\n").unwrap_err();
        assert_eq!(e.to_string(), "f.rs:1: section `x` starts and never ends");
        let e = sections("f.rs", "x\n// --8<-- [end:y]\n").unwrap_err();
        assert_eq!(e.to_string(), "f.rs:2: section `y` ends without a start");
    }
}
