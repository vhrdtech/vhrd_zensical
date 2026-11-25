//! The shared fixtures (`tests/fixtures`), which the package's Python tests read too.

use std::path::{Path, PathBuf};
use vz_core::abbreviations::{Abbreviations, AbbreviationsError};
use vz_core::code_index::{FORMAT, Index, Kind};
use vz_core::ext::{CODE_REFS_EXTENSION, Settings};
use vz_core::glossary::{Glossary, GlossaryError};
use vz_core::sources::{Sources, SourcesError};

fn fixtures(format: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(format)
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).expect("a fixture is readable text")
}

/// Each `err-*.toml` of a format with its error kind: the file name's part after `err-` that names a kind
fn error_files(format: &str, kinds: &[&'static str]) -> Vec<(PathBuf, &'static str)> {
    let mut files: Vec<(PathBuf, &'static str)> = std::fs::read_dir(fixtures(format))
        .expect("the fixtures folder is there")
        .map(|e| e.expect("a fixture's entry is readable").path())
        .filter_map(|p| {
            let name = p.file_stem()?.to_str()?.strip_prefix("err-")?.to_owned();
            let kind = kinds
                .iter()
                .find(|k| name == **k || name.starts_with(&format!("{k}-")))
                .unwrap_or_else(|| panic!("{}: no error kind of {kinds:?}", p.display()));
            Some((p, *kind))
        })
        .collect();
    files.sort();
    for kind in kinds {
        assert!(
            files.iter().any(|(_, k)| k == kind),
            "{format}: no fixture for error kind {kind}"
        );
    }
    files
}

#[test]
fn the_glossary_fixture_reads() {
    let g = Glossary::parse(&read(&fixtures("glossary").join("ok.toml"))).unwrap();
    let names: Vec<&str> = g.terms.iter().map(|t| t.name.as_str()).collect();
    assert_eq!(names, ["piece", "CRC", "tape"]);
    assert_eq!(g.find("PIECES").unwrap().name, "piece");
    assert_eq!(
        g.find("piece").unwrap().chapter.as_deref(),
        Some("../pieces/")
    );
    let crc = g.find("crc").unwrap();
    assert_eq!(crc.long.as_deref(), Some("cyclic redundancy check"));
    assert_eq!(crc.short, "A checksum over a piece.");
    assert_eq!(g.find("tape").unwrap().forms.len(), 0);
}

#[test]
fn every_glossary_error_fixture_fails_with_its_kind() {
    let kinds = ["parse", "blank-form", "chapter", "two-terms"];
    for (path, kind) in error_files("glossary", &kinds) {
        let e = Glossary::parse(&read(&path)).unwrap_err();
        let got = match e {
            GlossaryError::TooLarge => "too-large",
            GlossaryError::Parse(_) => "parse",
            GlossaryError::BlankForm { .. } => "blank-form",
            GlossaryError::Chapter { .. } => "chapter",
            GlossaryError::TwoTerms { .. } => "two-terms",
        };
        assert_eq!(got, kind, "{}", path.display());
    }
}

#[test]
fn the_abbreviations_fixture_reads() {
    let a = Abbreviations::parse(&read(&fixtures("abbreviations").join("ok.toml"))).unwrap();
    assert_eq!(
        a.long_forms.keys().collect::<Vec<_>>(),
        ["CAN", "CRC", "I/O", "eMMC"]
    );
    assert_eq!(a.long_form("eMMC"), Some("embedded MultiMediaCard"));
    assert_eq!(a.long_form("emmc"), None);
}

#[test]
fn every_abbreviations_error_fixture_fails_with_its_kind() {
    let kinds = ["parse", "abbreviation", "long-form"];
    for (path, kind) in error_files("abbreviations", &kinds) {
        let e = Abbreviations::parse(&read(&path)).unwrap_err();
        let got = match e {
            AbbreviationsError::TooLarge => "too-large",
            AbbreviationsError::Parse(_) => "parse",
            AbbreviationsError::Abbreviation { .. } => "abbreviation",
            AbbreviationsError::LongForm { .. } => "long-form",
        };
        assert_eq!(got, kind, "{}", path.display());
    }
}

#[test]
fn the_sources_fixture_reads() {
    let s = Sources::parse(&read(&fixtures("sources").join("ok.toml"))).unwrap();
    let names: Vec<&str> = s.sources.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, ["ww_stdlib", "shrink_wrap"]);
    let la = s.source_of("std_library/ww_logic_analyzer.md").unwrap();
    assert_eq!(la.crates, ["ww_logic_analyzer"]);
    assert_eq!(la.on_crates_io, ["ww_logic_analyzer"]);
    assert_eq!(la.root, "../ww_stdlib");
    assert_eq!(s.source_of("serdes/index.md").unwrap().crates.len(), 0);
    assert!(s.source_of("index.md").is_none());
}

#[test]
fn every_sources_error_fixture_fails_with_its_kind() {
    let kinds = [
        "parse",
        "format",
        "name",
        "two-sources",
        "root",
        "repository",
        "commit",
        "no-pages",
        "page-twice",
        "page",
    ];
    for (path, kind) in error_files("sources", &kinds) {
        let e = Sources::parse(&read(&path)).unwrap_err();
        let got = match e {
            SourcesError::TooLarge => "too-large",
            SourcesError::TooMany => "too-many",
            SourcesError::Parse(_) => "parse",
            SourcesError::Format { .. } => "format",
            SourcesError::Name { .. } => "name",
            SourcesError::TwoSources { .. } => "two-sources",
            SourcesError::Root { .. } => "root",
            SourcesError::Repository { .. } => "repository",
            SourcesError::Commit { .. } => "commit",
            SourcesError::NoPages { .. } => "no-pages",
            SourcesError::Page { .. } => "page",
            SourcesError::PageTwice { .. } => "page-twice",
        };
        assert_eq!(got, kind, "{}", path.display());
    }
}

#[test]
fn the_code_index_snapshot_holds_every_field_and_kind() {
    let text = read(&fixtures("code_index.json"));
    let index: Index = serde_json::from_str(&text).unwrap();
    assert_eq!(index.format, FORMAT);
    assert_eq!(index.sources["lib"].items[0].path, "lib::Chunk");
    let written: serde_json::Value = serde_json::to_value(&index).unwrap();
    let snapshot: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(written, snapshot, "the layout differs from the snapshot");
    let index = index.book;
    for kind in [
        Kind::Fn,
        Kind::Struct,
        Kind::Union,
        Kind::Field,
        Kind::Enum,
        Kind::Variant,
        Kind::Trait,
        Kind::Type,
        Kind::Const,
        Kind::Static,
        Kind::Macro,
    ] {
        assert!(
            index.items.iter().any(|i| i.kind == kind),
            "no item of {kind:?}"
        );
        assert_eq!(serde_json::to_value(kind).unwrap(), kind.name());
    }
    assert!(index.items.iter().any(|i| i.cfg.is_some()));
    assert!(index.items.iter().any(|i| i.reexports.is_some()));
    assert!(index.files.iter().any(|f| f.problem.is_some()));
    assert!(index.files.iter().any(|f| !f.sections.is_empty()));
}

#[test]
fn a_reader_ignores_fields_it_does_not_know() {
    let index: Index = serde_json::from_str(
        r#"{"format": 3, "book": {"commit": "c", "blob": "b", "items": [], "files": [], "more": 2}, "later": 1}"#,
    )
    .unwrap();
    assert_eq!(index.book.crates.len(), 0);
    assert!(index.sources.is_empty());
}

#[test]
fn a_zensical_toml_yields_the_settings() {
    let book: toml::Table = r#"
[project.markdown_extensions]
abbr = {}
"vhrd_zensical.code_refs" = { crates = ["tape"], on_crates_io = ["tape-core"] }
"#
    .parse()
    .unwrap();
    let ext = book["project"]["markdown_extensions"].as_table().unwrap();
    let s = Settings::from_toml(ext).unwrap();
    let c = s.code_refs.unwrap();
    assert_eq!(c.on_crates_io, ["tape-core"]);
    assert!(ext.contains_key(CODE_REFS_EXTENSION));
}
