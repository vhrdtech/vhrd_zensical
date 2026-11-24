//! A file regenerated at every build: compares before writing, and `--check` never writes, only reports.

use std::path::{Path, PathBuf};

/// What happened to a file
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Did {
    /// It already held the content
    Checked,
    /// It was written
    Wrote,
    /// `--check`: it would have been written
    Stale,
}

/// One file a command checked or wrote
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Written {
    /// What happened to it
    pub(crate) did: Did,
    /// Its path
    pub(crate) path: PathBuf,
    /// What it holds, for the report (`1711 items`)
    pub(crate) what: String,
}

/// Writes `content` to `path` unless it already holds it; with `check`, never writes, only reports whether it is
/// stale.
///
/// # Errors
///
/// `path` cannot be read back after writing, or (without `check`) its directory or the file cannot be written.
pub(crate) fn write_checked(
    path: &Path,
    content: &str,
    what: String,
    check: bool,
) -> std::io::Result<Written> {
    if std::fs::read_to_string(path).ok().as_deref() == Some(content) {
        return Ok(Written {
            did: Did::Checked,
            path: path.to_owned(),
            what,
        });
    }
    if check {
        return Ok(Written {
            did: Did::Stale,
            path: path.to_owned(),
            what,
        });
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, content)?;
    Ok(Written {
        did: Did::Wrote,
        path: path.to_owned(),
        what,
    })
}

/// Prints one file's report line, coloured by what happened to it; the one place a command prints its report
#[expect(
    clippy::print_stdout,
    reason = "a command's report is its own output, printed in this one place"
)]
pub(crate) fn report(paint: vc_ui::Paint, w: &Written) {
    let word = match w.did {
        Did::Checked => paint.good("checked"),
        Did::Wrote => paint.good("wrote"),
        Did::Stale => paint.warn("stale"),
    };
    // spirit-level: allow(print-in-library) vz_cli carries the vz binary's whole logic (AGENTS.md section 5);
    // this is a command's own report, printed in this one place
    println!(
        "{word} {} {}",
        w.path.display(),
        paint.dim(&format!("({})", w.what))
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_once_and_checks_after() {
        let dir = std::env::temp_dir().join(format!("vz_cli_written_test_{}", std::process::id()));
        let path = dir.join("a/b.txt");
        let w = write_checked(&path, "hello", "5 bytes".to_owned(), false).unwrap();
        assert_eq!(w.did, Did::Wrote);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "hello");
        let w = write_checked(&path, "hello", "5 bytes".to_owned(), false).unwrap();
        assert_eq!(w.did, Did::Checked);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn check_never_writes() {
        let dir =
            std::env::temp_dir().join(format!("vz_cli_written_test_check_{}", std::process::id()));
        let path = dir.join("a.txt");
        let w = write_checked(&path, "hello", "w".to_owned(), true).unwrap();
        assert_eq!(w.did, Did::Stale);
        assert!(!path.exists());
    }
}
