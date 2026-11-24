//! The cargo workspace a book documents, as far as its code index needs it.
//!
//! `cargo metadata --no-deps` gives the members and their library targets; each package's `.rs` files are read
//! from its directory; git gives the tracked files and the commit. Every program runs with the environment of an
//! outer cargo removed, so a run under `cargo test` or a build script sees the workspace as a person would.

use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Bytes of a source or text file read at most; a longer source file is an error, a longer text file is left out
// spirit-level: allow(exact-units-quantity) a byte count passed to take(), next to std's u64 sizes
pub(crate) const FILE_BYTES_MAX: u64 = 16 << 20;

/// A library target of a workspace member: a crate the index can walk
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Library {
    /// The library's own name, the first segment of its items' paths (`ww_tape`)
    pub name: String,
    /// Its package's name (`ww-tape`)
    pub package: String,
    /// Its package's directory relative to the workspace root, empty for a root package
    pub dir: String,
    /// Its root file relative to the package's directory (`src/lib.rs`)
    pub root: String,
}

impl Library {
    /// A path inside its package, relative to the workspace root
    fn path(&self, in_package: &str) -> String {
        if self.dir.is_empty() {
            in_package.to_owned()
        } else {
            format!("{}/{in_package}", self.dir)
        }
    }
}

/// A workspace: its root and the library targets of its members
#[derive(Clone, Debug)]
pub struct Workspace {
    /// The workspace root, absolute; every relative path starts here
    pub root: PathBuf,
    /// The members' library targets, proc-macros included, ordered by package directory
    pub libraries: Vec<Library>,
}

/// Why a workspace cannot be loaded
#[derive(Debug)]
pub enum LoadError {
    /// cargo cannot be started
    Cargo(std::io::Error),
    /// `cargo metadata` failed; its error output
    Metadata(String),
    /// cargo's metadata does not parse
    Parse(cargo_metadata::Error),
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cargo(e) => write!(f, "cannot run cargo: {e}"),
            Self::Metadata(e) => write!(f, "cargo metadata: {e}"),
            Self::Parse(e) => write!(f, "cargo metadata's output: {e}"),
        }
    }
}

impl std::error::Error for LoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Cargo(e) => Some(e),
            Self::Parse(e) => Some(e),
            Self::Metadata(_) => None,
        }
    }
}

/// Why git cannot answer
#[derive(Debug)]
pub enum GitError {
    /// git cannot be started
    Run(std::io::Error),
    /// git failed; the first line of its error output
    Failed(String),
}

impl std::fmt::Display for GitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Run(e) => write!(f, "cannot run git: {e}"),
            Self::Failed(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for GitError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Run(e) => Some(e),
            Self::Failed(_) => None,
        }
    }
}

impl Workspace {
    /// The workspace whose manifest is in `dir` or above it, as cargo finds it
    ///
    /// # Errors
    ///
    /// cargo cannot run, fails, or answers what does not parse.
    pub fn load(dir: &Path) -> Result<Self, LoadError> {
        let out = command("cargo", dir)
            .args(["metadata", "--format-version", "1", "--no-deps"])
            .output()
            .map_err(LoadError::Cargo)?;
        if !out.status.success() {
            return Err(LoadError::Metadata(
                String::from_utf8_lossy(&out.stderr).trim().to_owned(),
            ));
        }
        let meta = cargo_metadata::MetadataCommand::parse(String::from_utf8_lossy(&out.stdout))
            .map_err(LoadError::Parse)?;
        Ok(Self::from_metadata(&meta))
    }

    fn from_metadata(meta: &cargo_metadata::Metadata) -> Self {
        let root: PathBuf = meta.workspace_root.clone().into();
        let mut libraries = Vec::new();
        for p in meta.workspace_packages() {
            let manifest: &Path = p.manifest_path.as_std_path();
            let pkg_dir = manifest.parent().unwrap_or(&root);
            let dir = rel_path(&root, pkg_dir);
            for t in p.targets.iter().filter(|t| is_library(t)) {
                libraries.push(Library {
                    name: t.name.clone(),
                    package: p.name.to_string(),
                    dir: dir.clone(),
                    root: rel_path(pkg_dir, t.src_path.as_std_path()),
                });
            }
        }
        libraries.sort_by(|a, b| a.dir.cmp(&b.dir));
        Self { root, libraries }
    }

    /// The `.rs` files of a library's package, sorted by path: every one under the package's directory, leaving out
    /// `target/`, hidden directories and nested packages (a directory with its own `Cargo.toml`, like a test
    /// fixture)
    ///
    /// # Errors
    ///
    /// A file cannot be read, is not UTF-8, or is longer than [`FILE_BYTES_MAX`].
    pub(crate) fn sources(&self, lib: &Library) -> Result<Vec<SourceFile>, SourceError> {
        let dir = self.root.join(&lib.dir);
        let mut files = Vec::new();
        walk(&dir, &dir, &mut files);
        files.sort();
        files
            .into_iter()
            .map(|in_package| {
                let rel = lib.path(&in_package);
                let err = |source| SourceError {
                    file: rel.clone(),
                    source,
                };
                let bytes = read_bounded(&self.root.join(&rel))
                    .map_err(err)?
                    .ok_or_else(|| {
                        err(std::io::Error::new(
                            std::io::ErrorKind::FileTooLarge,
                            format!("longer than {FILE_BYTES_MAX} bytes"),
                        ))
                    })?;
                let text = String::from_utf8(bytes)
                    .map_err(|e| err(std::io::Error::new(std::io::ErrorKind::InvalidData, e)))?;
                Ok(SourceFile {
                    rel,
                    in_package,
                    text,
                })
            })
            .collect()
    }

    /// The files git tracks under the root, relative to it (`git ls-files`)
    ///
    /// # Errors
    ///
    /// The root is in no git checkout, or git cannot run.
    pub fn tracked(&self) -> Result<Vec<String>, GitError> {
        Ok(git(&self.root, &["ls-files", "-z"])?
            .split('\0')
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .collect())
    }

    /// The full hash of the commit checked out (`git rev-parse HEAD`)
    ///
    /// # Errors
    ///
    /// The root is in no git checkout, it has no commit yet, or git cannot run.
    pub fn head(&self) -> Result<String, GitError> {
        Ok(git(&self.root, &["rev-parse", "HEAD"])?.trim().to_owned())
    }
}

/// One `.rs` file of a package
#[derive(Debug)]
pub(crate) struct SourceFile {
    /// Its path relative to the workspace root
    pub(crate) rel: String,
    /// Its path relative to the package's directory (`src/a/b.rs`)
    pub(crate) in_package: String,
    /// Its text
    pub(crate) text: String,
}

/// A source file that cannot be read
#[derive(Debug)]
pub struct SourceError {
    /// The file, relative to the workspace root
    pub file: String,
    /// Why
    pub source: std::io::Error,
}

impl std::fmt::Display for SourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.file, self.source)
    }
}

impl std::error::Error for SourceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}

/// A file's bytes, at most [`FILE_BYTES_MAX`]; None when it is longer
pub(crate) fn read_bounded(path: &Path) -> std::io::Result<Option<Vec<u8>>> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(FILE_BYTES_MAX + 1)
        .read_to_end(&mut bytes)?;
    Ok((u64::try_from(bytes.len()).unwrap_or(u64::MAX) <= FILE_BYTES_MAX).then_some(bytes))
}

/// Whether a target is a library: a Rust library of any crate type, or a proc-macro
fn is_library(t: &cargo_metadata::Target) -> bool {
    use cargo_metadata::TargetKind as K;
    t.kind.iter().any(|k| {
        matches!(
            k,
            K::Lib | K::RLib | K::DyLib | K::CDyLib | K::StaticLib | K::ProcMacro
        )
    })
}

/// Runs git in `dir`: its output, or the first line of its error output
fn git(dir: &Path, args: &[&str]) -> Result<String, GitError> {
    let out = command("git", dir)
        .args(args)
        .output()
        .map_err(GitError::Run)?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    } else {
        let err = String::from_utf8_lossy(&out.stderr);
        Err(GitError::Failed(
            err.lines().next().unwrap_or_default().trim().to_owned(),
        ))
    }
}

/// A program started in `dir` with the environment of an outer cargo removed.
///
/// Removed are `CARGO_*` but `CARGO_HOME` and the registry and network settings, `CARGO`, `RUSTC*`, `RUSTFLAGS` and
/// the `RUSTUP_TOOLCHAIN` rustup sets for the outer cargo, so the workspace's own toolchain answers.
fn command(program: &str, dir: &Path) -> Command {
    let mut cmd = Command::new(program);
    cmd.current_dir(dir);
    for (k, _) in std::env::vars_os() {
        let Some(k) = k.to_str() else { continue };
        if is_outer_cargo(k) {
            cmd.env_remove(k);
        }
    }
    cmd
}

/// Whether an environment variable belongs to an outer cargo or rustup, not to the person
fn is_outer_cargo(k: &str) -> bool {
    let keep = k == "CARGO_HOME"
        || k.starts_with("CARGO_REGISTRIES_")
        || k.starts_with("CARGO_NET_")
        || k.starts_with("CARGO_HTTP_");
    (k.starts_with("CARGO_") && !keep)
        || k == "CARGO"
        || k.starts_with("RUSTC")
        || k == "RUSTFLAGS"
        || k == "RUSTUP_TOOLCHAIN"
}

/// `p` relative to `root` with `/` separators; `p` itself when it is not under `root`
fn rel_path(root: &Path, p: &Path) -> String {
    p.strip_prefix(root)
        .unwrap_or(p)
        .to_string_lossy()
        .replace('\\', "/")
}

/// Collects the `.rs` files under `dir`, relative to `pkg_dir`
fn walk(pkg_dir: &Path, dir: &Path, out: &mut Vec<String>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        let name = e.file_name();
        let name = name.to_string_lossy();
        let Ok(ft) = e.file_type() else { continue };
        if ft.is_dir() {
            if name.starts_with('.') || name == "target" || p.join("Cargo.toml").exists() {
                continue;
            }
            walk(pkg_dir, &p, out);
        } else if name.ends_with(".rs") {
            out.push(rel_path(pkg_dir, &p));
        }
    }
}

#[cfg(test)]
mod tests;
