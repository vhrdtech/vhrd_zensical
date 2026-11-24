use super::*;
use std::path::Path;

const FORGE: Forge<'static> = Forge {
    repository: "https://github.com/o/r/",
    commit: "abc123",
};

fn project() -> Project {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/projects/refs");
    Project::load(&dir).expect("the fixture project loads")
}

fn find<'a>(index: &'a Index, path: &str) -> &'a Entry {
    index
        .items
        .iter()
        .find(|e| e.path == path)
        .unwrap_or_else(|| {
            panic!(
                "{path} in {:#?}",
                index.items.iter().map(|e| &e.path).collect::<Vec<_>>()
            )
        })
}

#[test]
fn walks_the_library_module_tree() {
    let index = index(&project(), &[], &[], &[], FORGE).unwrap();
    let paths: Vec<&str> = index.items.iter().map(|e| e.path.as_str()).collect();
    assert_eq!(
        paths,
        [
            "tape::Bits",
            "tape::Bits::word",
            "tape::Outcome",
            "tape::Outcome::Done",
            "tape::Outcome::Partial",
            "tape::PIECE_BYTES_MAX",
            "tape::TapeWriter",
            "tape::TapeWriter::written",
            "tape::Writer",
            "tape::Writer::written",
            "tape::hello",
            "tape::hidden::reachable",
            "tape::inline::deep",
            "tape::level",
            "tape::level",
            "tape::moved::Sink",
            "tape::moved::Sink::SIDES",
            "tape::moved::Sink::flush",
            "tape::moved::Sink::put",
            "tape::platform",
            "tape::platform",
            "tape::quick",
            "tape::resume::TapeWriter::resume_on",
            "tape::writer::Outcome",
            "tape::writer::Outcome::Done",
            "tape::writer::Outcome::Partial",
            "tape::writer::TapeWriter",
            "tape::writer::TapeWriter::written",
            "tape::writer::resume::TapeWriter::resume_on",
        ]
    );
    assert_eq!(index.format, FORMAT);
    assert_eq!(index.commit, "abc123");
}

#[test]
fn an_entry_has_its_lines_signature_docs_and_address() {
    let index = index(&project(), &["tape".to_owned()], &[], &[], FORGE).unwrap();
    let c = find(&index, "tape::PIECE_BYTES_MAX");
    assert_eq!(c.kind, Kind::Const);
    assert_eq!(c.file, "src/lib.rs");
    assert_eq!(c.lines, [14, 14]);
    assert_eq!(c.source, "pub const PIECE_BYTES_MAX: usize = 4096;");
    assert_eq!(c.doc, "The largest piece.\n\nMore text, not in the card.");
    assert_eq!(
        c.url,
        "https://github.com/o/r/blob/abc123/src/lib.rs#L14-L14"
    );

    let r = find(&index, "tape::writer::resume::TapeWriter::resume_on");
    assert_eq!(r.kind, Kind::Fn);
    assert_eq!(r.lines, [8, 25]);
    assert_eq!(r.source, "pub fn resume_on(self, at: u64) -> Self");
    assert_eq!(
        r.doc,
        "Reopen the unfinished piece and go on writing it.\n\nSecond paragraph."
    );

    let s = find(&index, "tape::writer::TapeWriter");
    assert_eq!(
        s.source,
        "pub struct TapeWriter {\n    /// Bytes written so far.\n    pub written: u64,\n    kept: u64,\n}"
    );
}

#[test]
fn a_reexport_has_the_item_s_lines_under_its_own_path() {
    let index = index(&project(), &[], &[], &[], FORGE).unwrap();
    let alias = find(&index, "tape::Writer");
    let item = find(&index, "tape::writer::TapeWriter");
    assert_eq!(
        (alias.kind, &alias.file, alias.lines),
        (item.kind, &item.file, item.lines)
    );
    assert_eq!(alias.url, item.url);
    assert_eq!(alias.reexports.as_deref(), Some("tape::writer::TapeWriter"));
    assert_eq!(item.reexports, None);
}

#[test]
fn trait_items_unions_and_cfg_arms_are_listed() {
    let index = index(&project(), &[], &[], &[], FORGE).unwrap();
    assert_eq!(find(&index, "tape::moved::Sink::SIDES").kind, Kind::Const);
    let put = find(&index, "tape::moved::Sink::put");
    assert_eq!(
        (put.kind, put.source.as_str()),
        (Kind::Fn, "fn put(&mut self, b: &[u8]);")
    );
    assert_eq!(find(&index, "tape::Bits").kind, Kind::Union);
    let arms: Vec<Option<&str>> = index
        .items
        .iter()
        .filter(|e| e.path == "tape::platform")
        .map(|e| e.cfg.as_deref())
        .collect();
    assert_eq!(arms, [Some("unix"), Some("not(unix)")]);
    let kinds: Vec<Kind> = index
        .items
        .iter()
        .filter(|e| e.path == "tape::level")
        .map(|e| e.kind)
        .collect();
    assert_eq!(kinds, [Kind::Struct, Kind::Fn]);
}

#[test]
fn an_attribute_on_the_item_s_line_is_part_of_it() {
    let index = index(&project(), &[], &[], &[], FORGE).unwrap();
    let q = find(&index, "tape::quick");
    assert_eq!(q.source, "#[inline] pub fn quick() {}");
    assert_eq!(q.lines[0], q.lines[1]);
}

#[test]
fn an_unknown_crate_names_the_libraries() {
    let e = index(&project(), &["tape_core".to_owned()], &[], &[], FORGE).unwrap_err();
    assert_eq!(
        e.to_string(),
        "`tape_core` is no library of the workspace; the libraries are tape"
    );
}

#[test]
fn source_cuts_a_long_body() {
    let lines: Vec<&str> = "    pub fn f(\n        a: u8,\n    ) {\n        x\n    }"
        .lines()
        .collect();
    assert_eq!(
        source(&lines, 1, 5, Some(3)),
        "pub fn f(\n    a: u8,\n) {\n    x\n}"
    );
    let long: Vec<String> = std::iter::once("pub fn f() {".to_owned())
        .chain((0..20).map(|i| format!("    {i};")))
        .chain(std::iter::once("}".to_owned()))
        .collect();
    let long: Vec<&str> = long.iter().map(String::as_str).collect();
    assert_eq!(source(&long, 1, 22, Some(1)), "pub fn f()");
}

#[test]
fn the_extension_reads_this_format() {
    let py = include_str!("../../../../python/spl_book_ext/src/spl_book_ext/code_refs.py");
    assert!(
        py.contains(&format!("\nFORMAT = {FORMAT}\n")),
        "spl_book_ext's FORMAT differs from {FORMAT}"
    );
}

#[test]
fn lists_the_text_files_with_their_sections() {
    let index = index(&project(), &[], &["app"], &[], FORGE).unwrap();
    let paths: Vec<&str> = index.files.iter().map(|f| f.path.as_str()).collect();
    assert!(paths.contains(&"src/writer/resume.rs"), "{paths:?}");
    assert!(!paths.iter().any(|p| p.starts_with("app/")), "{paths:?}");
    let resume = index
        .files
        .iter()
        .find(|f| f.path == "src/writer/resume.rs")
        .unwrap();
    assert_eq!(resume.sections.get("call"), Some(&[11, 12]));
    let broken = index
        .files
        .iter()
        .find(|f| f.path == "src/broken_marker.txt")
        .unwrap();
    assert_eq!(
        broken.problem.as_deref(),
        Some("src/broken_marker.txt:2: section `open` starts and never ends")
    );
    assert!(broken.sections.is_empty());
    assert_eq!(index.blob, "https://github.com/o/r/blob/abc123/");
}

#[test]
fn a_library_has_its_package_name_and_is_published_when_declared() {
    let listed = ["tape-core".to_owned()];
    let index = index(&project(), &[], &[], &listed, FORGE).unwrap();
    assert_eq!(
        index.crates.get("tape"),
        Some(&CrateEntry {
            package: "tape-core".to_owned(),
            published: true
        })
    );
    let index = index_without(&project());
    assert_eq!(index.crates.get("tape").map(|c| c.published), Some(false));
}

fn index_without(p: &Project) -> Index {
    index(p, &[], &[], &[], FORGE).unwrap()
}
