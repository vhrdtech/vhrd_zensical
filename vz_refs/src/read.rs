//! A book's files read whole, bounded.

use std::io::Read as _;
use std::path::Path;

/// The text of the file at `path`, at most `bytes_max` bytes
///
/// # Errors
///
/// The file cannot be read, is longer than `bytes_max` or is not UTF-8.
// spirit-level: allow(exact-units-quantity) a byte count passed to take(), next to std's u64 sizes
pub fn text(path: &Path, bytes_max: u64) -> std::io::Result<String> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(bytes_max.saturating_add(1))
        .read_to_end(&mut bytes)?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > bytes_max {
        return Err(std::io::Error::new(
            std::io::ErrorKind::FileTooLarge,
            format!("longer than {bytes_max} bytes"),
        ));
    }
    String::from_utf8(bytes).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

/// A book format's file read and parsed, or why not; its message names the file
#[derive(Debug)]
pub struct FileError<E> {
    /// The file, as the settings name it
    pub file: String,
    /// Why
    pub kind: FileErrorKind<E>,
}

/// Why a book format's file is refused
#[derive(Debug)]
pub enum FileErrorKind<E> {
    /// It cannot be read
    Read(std::io::Error),
    /// It does not parse
    Parse(E),
}

impl<E: std::fmt::Display> std::fmt::Display for FileError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            FileErrorKind::Read(e) => write!(f, "{}: {e}", self.file),
            FileErrorKind::Parse(e) => write!(f, "{}: {e}", self.file),
        }
    }
}

impl<E: std::error::Error + 'static> std::error::Error for FileError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.kind {
            FileErrorKind::Read(e) => Some(e),
            FileErrorKind::Parse(e) => Some(e),
        }
    }
}

/// The book format's file at `path`, read bounded by [`vz_core::FILE_BYTES_MAX`] and parsed by `parse`; errors
/// name it `file`
///
/// # Errors
///
/// The file cannot be read or does not parse.
pub fn format<T, E>(
    path: &Path,
    file: &str,
    parse: impl FnOnce(&str) -> Result<T, E>,
) -> Result<T, FileError<E>> {
    let err = |kind| FileError {
        file: file.to_owned(),
        kind,
    };
    let text =
        text(path, vz_core::FILE_BYTES_MAX as u64).map_err(|e| err(FileErrorKind::Read(e)))?;
    parse(&text).map_err(|e| err(FileErrorKind::Parse(e)))
}
