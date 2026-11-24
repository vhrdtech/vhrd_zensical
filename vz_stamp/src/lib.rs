//! The book's footer build stamp: crate version, git commit and build time, and the script that turns it into
//! the build's age from the reader's own clock.
//!
//! `vz stamp` reads a project's `Cargo.toml` for its name and version ([`package`]), builds a [`Stamp`] with the
//! git SHA the caller passes (`vc_build::collect`, in `vz_cli`: this crate stays off `vc_build`), and writes its
//! [`Stamp::html`] to the stamp partial and [`AGE_SCRIPT`] to the age script, both at [`vz_core::stamp`]'s
//! default paths unless the book's settings name others.

use std::fmt;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// Reading another project's `Cargo.toml` at run time.
#[derive(Debug)]
pub enum Error {
    /// `Cargo.toml` could not be read.
    ReadManifest(std::io::Error),
    /// `Cargo.toml` is not valid TOML.
    ParseManifest(toml::de::Error),
    /// No `[package].name` and no name override.
    MissingName,
    /// Neither `[package].version` nor `[workspace.package].version`.
    MissingVersion,
    /// A virtual workspace with no `[workspace.package].version` and no member named so.
    MissingMember(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReadManifest(_) => write!(f, "reading Cargo.toml"),
            Self::ParseManifest(_) => write!(f, "parsing Cargo.toml"),
            Self::MissingName => write!(
                f,
                "Cargo.toml has no [package].name: pass --name for a virtual workspace"
            ),
            Self::MissingVersion => write!(
                f,
                "Cargo.toml has neither [package].version nor [workspace.package].version"
            ),
            Self::MissingMember(name) => write!(
                f,
                "Cargo.toml has no [workspace.package].version and no workspace member with [package].name \"{name}\""
            ),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ReadManifest(e) => Some(e),
            Self::ParseManifest(e) => Some(e),
            Self::MissingName | Self::MissingVersion | Self::MissingMember(_) => None,
        }
    }
}

/// The crate name and version a stamp shows, read from `repo`'s `Cargo.toml`.
///
/// `[package].name` (or `name_override` for a virtual workspace, which has none) and `[package].version`,
/// falling back to `[workspace.package].version` for a member that inherits it (`version.workspace = true`). A
/// virtual workspace with `name_override` takes the version of the member whose `[package].name` is that name
/// (`version.workspace = true` there resolves to `[workspace.package].version`); with no such member it uses
/// `[workspace.package].version`.
///
/// # Errors
///
/// `Cargo.toml` is missing or not valid TOML, it has no `[package].name` and `name_override` is `None`, it has
/// neither `[package].version` nor `[workspace.package].version` (and, for a virtual workspace, no member named
/// `name_override`).
pub fn package(repo: &Path, name_override: Option<&str>) -> Result<(String, String), Error> {
    let text = std::fs::read_to_string(repo.join("Cargo.toml")).map_err(Error::ReadManifest)?;
    let doc: toml::Table = text.parse().map_err(Error::ParseManifest)?;
    let package = doc.get("package").and_then(toml::Value::as_table);
    let workspace_package = doc
        .get("workspace")
        .and_then(|w| w.get("package"))
        .and_then(toml::Value::as_table);
    let str_field = |table: Option<&toml::map::Map<String, toml::Value>>, key: &str| {
        table
            .and_then(|t| t.get(key))
            .and_then(toml::Value::as_str)
            .map(str::to_owned)
    };
    let name = match name_override {
        Some(n) => n.to_owned(),
        None => str_field(package, "name").ok_or(Error::MissingName)?,
    };
    let workspace_version = str_field(workspace_package, "version");
    let version = match str_field(package, "version") {
        Some(v) => v,
        None if package.is_none() && name_override.is_some() => {
            match member_version(repo, &doc, &name)? {
                Some(MemberVersion::Own(v)) => v,
                Some(MemberVersion::Inherited) | None => {
                    workspace_version.ok_or_else(|| Error::MissingMember(name.clone()))?
                }
            }
        }
        None => workspace_version.ok_or(Error::MissingVersion)?,
    };
    Ok((name, version))
}

/// What a workspace member's `[package].version` says.
enum MemberVersion {
    /// A version of its own.
    Own(String),
    /// `version.workspace = true` or no version at all.
    Inherited,
}

/// The version of the workspace member named `name`, `None` when no member has that `[package].name`.
///
/// Members are literal paths or `dir/*` patterns, the forms workspaces use for crate directories.
fn member_version(
    repo: &Path,
    doc: &toml::Table,
    name: &str,
) -> Result<Option<MemberVersion>, Error> {
    let members = doc
        .get("workspace")
        .and_then(|w| w.get("members"))
        .and_then(toml::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(toml::Value::as_str);
    for member in members {
        let dirs = match member.strip_suffix("/*") {
            Some(parent) => std::fs::read_dir(repo.join(parent))
                .map(|rd| rd.filter_map(|e| e.ok().map(|e| e.path())).collect())
                .unwrap_or_default(),
            None => vec![repo.join(member)],
        };
        for dir in dirs {
            let Ok(text) = std::fs::read_to_string(dir.join("Cargo.toml")) else {
                continue;
            };
            let manifest: toml::Table = text.parse().map_err(Error::ParseManifest)?;
            let Some(package) = manifest.get("package").and_then(toml::Value::as_table) else {
                continue;
            };
            if package.get("name").and_then(toml::Value::as_str) != Some(name) {
                continue;
            }
            return Ok(Some(
                match package.get("version").and_then(toml::Value::as_str) {
                    Some(v) => MemberVersion::Own(v.to_owned()),
                    None => MemberVersion::Inherited,
                },
            ));
        }
    }
    Ok(None)
}

/// A build stamp: the pieces a reader needs to tell which build they are looking at.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stamp {
    /// The crate name the stamp shows.
    pub name: String,
    /// The crate version the stamp shows.
    pub version: String,
    /// Short git SHA, `-dirty` when the tree had changes; `unknown` without git (`vc_build::collect`, in the
    /// caller).
    pub sha: String,
    /// Unix seconds of the moment the book was built (not the commit time: a rebuild of the same commit later
    /// still ages from when it ran).
    pub built_unix: u64,
}

impl Stamp {
    /// `built_unix` set to now.
    #[must_use]
    pub fn now(name: String, version: String, sha: String) -> Self {
        let built_unix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        Self {
            name,
            version,
            sha,
            built_unix,
        }
    }

    /// The generated partial: a `<div class="book-stamp" data-built="...">` the reader's browser turns into an
    /// age (the age script's job) beside the absolute build time. Written to [`vz_core::stamp::DEFAULT_STAMP_PARTIAL`]
    /// (or the book's own name for it) under the theme's `custom_dir`.
    ///
    /// ```
    /// let s = vz_stamp::Stamp { name: "fx".into(), version: "1.2.3".into(), sha: "abc1234".into(), built_unix: 0 };
    /// assert_eq!(s.html(), "<div class=\"book-stamp\" data-built=\"0\">fx 1.2.3 \u{b7} abc1234 \u{b7} built 1970-01-01 00:00 UTC (<span class=\"book-stamp-age\">just built</span>)</div>\n");
    /// ```
    #[must_use]
    pub fn html(&self) -> String {
        let built: i64 = self.built_unix.try_into().unwrap_or(i64::MAX);
        let (y, m, d) = civil_date(built.div_euclid(86400));
        let rest = built.rem_euclid(86400);
        format!(
            "<div class=\"{class}\" {attr}=\"{built_unix}\">{name} {version} \u{b7} {sha} \u{b7} built \
             {y:04}-{m:02}-{d:02} {h:02}:{min:02} UTC (<span class=\"{age_class}\">just built</span>)</div>\n",
            class = vz_core::stamp::STAMP_CLASS,
            attr = vz_core::stamp::BUILT_ATTRIBUTE,
            built_unix = self.built_unix,
            name = escape(&self.name),
            version = escape(&self.version),
            sha = escape(&self.sha),
            age_class = vz_core::stamp::AGE_CLASS,
            h = rest / 3600,
            min = rest % 3600 / 60,
        )
    }
}

/// Escapes the handful of characters that matter inside an HTML text node; a crate name, version or SHA never
/// carries more than this.
fn escape(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '&' => "&amp;".to_owned(),
            '<' => "&lt;".to_owned(),
            '>' => "&gt;".to_owned(),
            '"' => "&quot;".to_owned(),
            c => c.to_string(),
        })
        .collect()
}

/// The civil date `(year, month, day)` of a day counted from 1970-01-01 (Howard Hinnant's algorithm); kept here
/// rather than taken from `vc_build` so this crate stays off it, as the crate map asks.
// spirit-level: allow(exact-units-quantity) a day count from the Unix epoch, ported from vc_build's identical
// algorithm (vc_build::civil_date); typing it would gain nothing here
fn civil_date(days: i64) -> (i64, u8, u8) {
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    // day is 1..=31 and month 1..=12 by the algorithm
    (year, month as u8, day as u8)
}

/// The age-rendering script.
///
/// Turns each stamp's absolute build time into "built N ago", coarser as it grows (seconds, minutes, hours,
/// days), beside the absolute UTC time already in the stamp. Generic and static (nothing here depends on a
/// particular build), but `vz stamp` rewrites it every run anyway so an update to the coarsening logic reaches
/// every book the next time its build recipe runs. Written to [`vz_core::stamp::DEFAULT_AGE_SCRIPT`] (or the
/// book's own name for it) under the docs folder; its first line carries [`vz_core::stamp::AGE_MARKER`].
pub const AGE_SCRIPT: &str = r#"// vz-stamp-age: written by `vz stamp`, turns each .book-stamp[data-built]'s build time into "built N ago",
// coarser as it grows (seconds, minutes, hours, days), beside the absolute UTC time already in the stamp.
(function () {
  function render(el) {
    var span = el.querySelector(".book-stamp-age");
    var built = Number.parseInt(el.dataset.built, 10) * 1000;
    if (!span || Number.isNaN(built)) {
      return;
    }
    var s = Math.max(0, Math.round((Date.now() - built) / 1000));
    var text;
    if (s < 60) {
      text = s + " s ago";
    } else if (s < 3600) {
      text = Math.floor(s / 60) + " min ago";
    } else if (s < 86400) {
      text = Math.floor(s / 3600) + " h ago";
    } else {
      text = Math.floor(s / 86400) + " days ago";
    }
    span.textContent = text;
  }
  function tick() {
    document.querySelectorAll(".book-stamp[data-built]").forEach(render);
  }
  tick();
  setInterval(tick, 30000);
})();
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_reads_name_and_version() {
        let dir =
            std::env::temp_dir().join(format!("vz_stamp_test_package_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("Cargo.toml"),
            "[package]\nname = \"exact_units\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        assert_eq!(
            package(&dir, None).unwrap(),
            ("exact_units".to_owned(), "0.1.0".to_owned())
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn version_falls_back_to_workspace_package() {
        let dir = std::env::temp_dir().join(format!("vz_stamp_test_wspkg_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("Cargo.toml"),
            "[workspace.package]\nversion = \"2.0.0\"\n\n[package]\nname = \"member\"\nversion.workspace = true\n",
        )
        .unwrap();
        assert_eq!(
            package(&dir, None).unwrap(),
            ("member".to_owned(), "2.0.0".to_owned())
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn virtual_workspace_needs_a_name_override() {
        let dir =
            std::env::temp_dir().join(format!("vz_stamp_test_virtual_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/*\"]\n\n[workspace.package]\nversion = \"0.4.15\"\n",
        )
        .unwrap();
        assert!(matches!(package(&dir, None), Err(Error::MissingName)));
        assert_eq!(
            package(&dir, Some("vhrd_zensical")).unwrap(),
            ("vhrd_zensical".to_owned(), "0.4.15".to_owned())
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn virtual_workspace_takes_the_named_member_s_version() {
        let dir = std::env::temp_dir().join(format!("vz_stamp_test_member_{}", std::process::id()));
        std::fs::create_dir_all(dir.join("vz_cli")).unwrap();
        std::fs::create_dir_all(dir.join("crates/other")).unwrap();
        std::fs::write(
            dir.join("Cargo.toml"),
            "[workspace]\nmembers = [\"vz_cli\", \"crates/*\"]\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("vz_cli/Cargo.toml"),
            "[package]\nname = \"vz_cli\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("crates/other/Cargo.toml"),
            "[package]\nname = \"other\"\nversion = \"9.9.9\"\n",
        )
        .unwrap();
        assert_eq!(
            package(&dir, Some("vz_cli")).unwrap(),
            ("vz_cli".to_owned(), "0.1.0".to_owned())
        );
        assert_eq!(
            package(&dir, Some("other")).unwrap(),
            ("other".to_owned(), "9.9.9".to_owned())
        );
        match package(&dir, Some("absent")) {
            Err(e @ Error::MissingMember(_)) => {
                let msg = e.to_string();
                assert!(msg.contains("[workspace.package].version"), "{msg}");
                assert!(msg.contains("absent"), "{msg}");
            }
            other => panic!("{other:?}"),
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn html_carries_the_markers_and_escapes_outside_text() {
        let s = Stamp {
            name: "<fx>".into(),
            version: "1.2.3".into(),
            sha: "abc1234-dirty".into(),
            built_unix: 1_791_000_000,
        };
        let html = s.html();
        assert!(html.contains(vz_core::stamp::STAMP_MARKER), "{html}");
        assert!(!html.contains(vz_core::stamp::AGE_MARKER), "{html}");
        assert!(html.contains("&lt;fx&gt; 1.2.3 \u{b7} abc1234-dirty \u{b7} built "));
        assert!(html.contains("data-built=\"1791000000\""));
    }

    #[test]
    fn age_script_carries_its_marker_and_nothing_of_the_stamp_s_html_marker_alone_would_miss() {
        let first_line = AGE_SCRIPT.lines().next().unwrap_or_default();
        assert!(
            first_line.contains(vz_core::stamp::AGE_MARKER),
            "{first_line}"
        );
        assert!(AGE_SCRIPT.contains(vz_core::stamp::STAMP_CLASS));
        assert!(AGE_SCRIPT.contains(vz_core::stamp::AGE_CLASS));
        assert!(AGE_SCRIPT.contains(vz_core::stamp::BUILT_ATTRIBUTE));
    }
}
