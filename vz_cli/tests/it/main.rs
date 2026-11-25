//! The `vz` binary as a person runs it.

use std::path::{Path, PathBuf};
use std::process::Command;

/// `vz` with the outer cargo's `CARGO_*` variables removed, so it sees the environment a person's shell gives
fn vz() -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_vz"));
    for (key, _) in std::env::vars_os() {
        if key.to_string_lossy().starts_with("CARGO_") {
            cmd.env_remove(key);
        }
    }
    cmd
}

#[test]
fn version_names_the_cargo_version() {
    let out = vz().arg("--version").output().unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        stdout.starts_with(&format!("vz {} (", env!("CARGO_PKG_VERSION"))),
        "{stdout}"
    );
}

#[test]
fn no_command_prints_help_and_fails() {
    let out = vz().arg("--color").arg("never").output().unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8(out.stderr).unwrap().contains("Usage: vz"));
}

#[test]
fn completion_registers_for_each_shell() {
    for shell in ["bash", "zsh", "fish"] {
        let out = vz().env("COMPLETE", shell).output().unwrap();
        assert!(out.status.success(), "{shell}");
        assert!(!out.stdout.is_empty(), "{shell}: no registration");
    }
}

/// A scratch book: `book_checks.toml` with a basic and an advanced part and a Reference shelf, a nav of
/// `nav` (a TOML array) and the pages `pages` (name, level, body)
fn frame_book(name: &str, nav: &str, waived: &str, pages: &[(&str, &str, &str)]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("vz_cli_it_check_{}_{name}", std::process::id()));
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(dir.join("docs")).expect("the scratch docs folder is writable");
    std::fs::write(
        dir.join("zensical.toml"),
        format!("[project]\nsite_name = \"t\"\nnav = {nav}\n"),
    )
    .expect("the scratch book is writable");
    std::fs::write(
        dir.join("book_checks.toml"),
        format!(
            "outside = [\"Reference\"]\n\n[[part]]\ntitle = \"Basic\"\nlevel = \"basic\"\n\n[[part]]\ntitle = \"Advanced\"\nlevel = \"advanced\"\n{waived}"
        ),
    )
    .expect("the scratch book is writable");
    for (page, level, body) in pages {
        std::fs::write(
            dir.join("docs").join(page),
            format!("---\nlevel: {level}\n---\n\n# Page\n\n{body}"),
        )
        .expect("the scratch page is writable");
    }
    dir
}

/// Runs `vz check` on `dir`: its exit code and stdout
fn vz_check(dir: &Path) -> (Option<i32>, String, String) {
    let out = vz()
        .args(["--color", "never", "check", "--root"])
        .arg(dir)
        .output()
        .expect("vz runs");
    (
        out.status.code(),
        String::from_utf8(out.stdout).expect("stdout is UTF-8"),
        String::from_utf8(out.stderr).expect("stderr is UTF-8"),
    )
}

const PAGES: [(&str, &str, &str); 3] = [
    ("a.md", "basic", ""),
    ("b.md", "advanced", ""),
    (
        "c.md",
        "basic",
        "```rust\n--8<-- \"src/lib.rs:part\"\n```\n",
    ),
];

#[test]
fn check_passes_a_frame_in_order() {
    let dir = frame_book(
        "ok",
        r#"[{ "Basic" = ["a.md", "c.md"] }, { "Advanced" = ["b.md"] }, { "Reference" = [] }]"#,
        "",
        &PAGES,
    );
    let (code, stdout, stderr) = vz_check(&dir);
    assert_eq!(code, Some(0), "{stdout}{stderr}");
    assert!(
        stdout.starts_with("checked ") && stdout.contains("(3 pages in 2 parts and 1 shelves)"),
        "{stdout}"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn check_fails_a_planted_step_down() {
    let dir = frame_book(
        "step_down",
        r#"[{ "Basic" = ["a.md"] }, { "Advanced" = ["b.md", "c.md"] }]"#,
        "",
        &PAGES,
    );
    let (code, stdout, stderr) = vz_check(&dir);
    assert_eq!(code, Some(1), "{stdout}{stderr}");
    assert_eq!(
        stdout,
        "problem c.md: a basic page in part `Advanced`, whose pages are advanced\n\
         problem c.md: the nav steps down from advanced (b.md) to basic\n"
    );
    assert_eq!(stderr, "vz: 2 problems in the book's frame\n");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn check_fails_a_planted_bare_fence() {
    let fenced = ("c.md", "basic", "Text.\n\n```rust\nfn main() {}\n```\n");
    let dir = frame_book(
        "bare_fence",
        r#"[{ "Basic" = ["a.md", "c.md"] }, { "Advanced" = ["b.md"] }]"#,
        "",
        &[PAGES[0], PAGES[1], fenced],
    );
    let (code, stdout, _) = vz_check(&dir);
    assert_eq!(code, Some(1), "{stdout}");
    assert_eq!(
        stdout,
        "problem c.md: 1 `rust` fences not an include of a compiled source (`--8<-- \"path:section\"`), 0 waived in book_checks.toml; lines 9\n"
    );
    std::fs::remove_dir_all(&dir).unwrap();
    // the same fence, waived
    let dir = frame_book(
        "bare_fence_waived",
        r#"[{ "Basic" = ["a.md", "c.md"] }, { "Advanced" = ["b.md"] }]"#,
        "\n[bare_fences]\n\"c.md\" = 1\n",
        &[PAGES[0], PAGES[1], fenced],
    );
    assert_eq!(vz_check(&dir).0, Some(0));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn check_fails_a_page_listed_twice() {
    let dir = frame_book(
        "twice",
        r#"[{ "Basic" = ["a.md", "c.md"] }, { "Advanced" = ["b.md"] }, { "Reference" = ["a.md"] }]"#,
        "",
        &PAGES,
    );
    let (code, stdout, _) = vz_check(&dir);
    assert_eq!(code, Some(1), "{stdout}");
    assert_eq!(stdout, "problem a.md: listed twice in the nav\n");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn check_without_a_frame_file_is_an_error() {
    let dir = frame_book("no_file", r#"[{ "Basic" = ["a.md"] }]"#, "", &PAGES);
    std::fs::remove_file(dir.join("book_checks.toml")).unwrap();
    let (code, _, stderr) = vz_check(&dir);
    assert_eq!(code, Some(1), "{stderr}");
    assert!(stderr.starts_with("vz: book_checks.toml: "), "{stderr}");
    std::fs::remove_dir_all(&dir).unwrap();
}
