use super::*;

fn fixture() -> Workspace {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/projects/refs");
    Workspace::load(&dir).expect("the fixture workspace loads")
}

#[test]
fn lists_the_library_targets_by_their_own_names() {
    let ws = fixture();
    assert_eq!(
        ws.libraries,
        [Library {
            name: "tape".to_owned(),
            package: "tape-core".to_owned(),
            dir: String::new(),
            root: "src/lib.rs".to_owned(),
        }]
    );
    assert!(ws.root.ends_with("tests/projects/refs"), "{ws:?}");
}

#[test]
fn reads_a_package_s_sources_without_nested_packages() {
    let ws = fixture();
    let lib = ws.libraries.first().expect("one library");
    let files: Vec<String> = ws
        .sources(lib)
        .unwrap()
        .into_iter()
        .map(|f| f.in_package)
        .collect();
    assert_eq!(
        files,
        [
            "src/hidden.rs",
            "src/lib.rs",
            "src/other/renamed.rs",
            "src/tests.rs",
            "src/writer/mod.rs",
            "src/writer/resume.rs",
        ]
    );
}

#[test]
fn git_names_the_tracked_files_and_the_commit() {
    let ws = fixture();
    let tracked = ws.tracked().unwrap();
    assert!(tracked.iter().any(|f| f == "src/lib.rs"), "{tracked:?}");
    assert!(tracked.iter().any(|f| f == "app/Cargo.toml"), "{tracked:?}");
    let head = ws.head().unwrap();
    assert_eq!(head.len(), 40, "{head}");
    assert!(head.bytes().all(|b| b.is_ascii_hexdigit()), "{head}");
}

#[test]
fn outside_a_workspace_cargo_s_error_is_kept() {
    let dir = std::env::temp_dir().join(format!("vz_refs_no_workspace_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let e = Workspace::load(&dir).unwrap_err();
    std::fs::remove_dir_all(&dir).unwrap();
    assert!(matches!(e, LoadError::Metadata(_)), "{e}");
    assert!(e.to_string().contains("Cargo.toml"), "{e}");
}

#[test]
fn an_outer_cargo_s_variables_are_removed() {
    for k in [
        "CARGO",
        "CARGO_MANIFEST_DIR",
        "CARGO_TARGET_DIR",
        "CARGO_PKG_NAME",
        "RUSTC",
        "RUSTC_WRAPPER",
        "RUSTFLAGS",
        "RUSTUP_TOOLCHAIN",
    ] {
        assert!(is_outer_cargo(k), "{k}");
    }
    for k in [
        "CARGO_HOME",
        "CARGO_REGISTRIES_CRATES_IO_PROTOCOL",
        "CARGO_NET_OFFLINE",
        "CARGO_HTTP_TIMEOUT",
        "PATH",
        "RUSTUP_HOME",
    ] {
        assert!(!is_outer_cargo(k), "{k}");
    }
    // nextest runs this under cargo: its variables are set here and gone in the child
    let removed: Vec<_> = command("cargo", Path::new("."))
        .get_envs()
        .filter(|(_, v)| v.is_none())
        .map(|(k, _)| k.to_owned())
        .collect();
    assert!(
        removed.iter().any(|k| k == "CARGO_MANIFEST_DIR"),
        "{removed:?}"
    );
}
