use super::*;
use crate::cards::tests::Plain;
use vz_core::code_index::CrateEntry;

const BLOB: &str = "https://github.com/o/r/blob/abc/";

fn item(path: &str, kind: Kind, first: usize) -> Entry {
    Entry {
        path: path.to_owned(),
        kind,
        cfg: None,
        file: "src/lib.rs".to_owned(),
        lines: [first, first + 6],
        source: format!("pub {} {}()", kind.name(), own_name(path)),
        doc: "Does it.".to_owned(),
        url: format!("{BLOB}src/lib.rs#L{first}-L{}", first + 6),
        reexports: None,
    }
}

fn reexport(path: &str, of: &Entry) -> Entry {
    Entry {
        path: path.to_owned(),
        reexports: Some(of.path.clone()),
        ..of.clone()
    }
}

fn with_cfg(mut e: Entry, cfg: &str) -> Entry {
    e.cfg = Some(cfg.to_owned());
    e
}

fn with_doc(mut e: Entry, doc: &str) -> Entry {
    doc.clone_into(&mut e.doc);
    e
}

fn index() -> Index {
    let writer = item("tape::writer::TapeWriter", Kind::Struct, 10);
    let definitions = item("tape::open::Definitions", Kind::Struct, 200);
    Index {
        format: FORMAT,
        commit: "abc".to_owned(),
        blob: BLOB.to_owned(),
        crates: BTreeMap::new(),
        items: vec![
            with_doc(
                item("tape::writer::TapeWriter::resume_on", Kind::Fn, 3),
                "Reopen it: see [`Recovery`], [`Self::flush`] and [`Gone`].",
            ),
            item("tape::writer::TapeWriter::flush", Kind::Fn, 40),
            reexport("tape::Writer", &writer),
            reexport("tape::TapeWriter", &writer),
            writer,
            item("tape::chain::resume", Kind::Fn, 50),
            item("tape::writer::resume", Kind::Fn, 60),
            item("tape_fs::writer::resume", Kind::Fn, 70),
            item("tape::hello", Kind::Macro, 80),
            item("tape::Recovery", Kind::Enum, 20),
            item("tape::level", Kind::Struct, 90),
            item("tape::level", Kind::Fn, 100),
            with_cfg(item("tape::platform", Kind::Fn, 110), "unix"),
            with_cfg(
                item("tape::platform", Kind::Fn, 120),
                "not(feature = \"std\")",
            ),
            reexport("tape::Definitions", &definitions),
            definitions,
            item("tape::other::Definitions", Kind::Struct, 210),
        ],
        files: vec![
            FileEntry {
                path: "tests/resume.rs".to_owned(),
                lines: 4,
                sections: [("call".to_owned(), [3, 3])].into(),
                problem: None,
            },
            FileEntry {
                path: "src/lib.rs".to_owned(),
                lines: 40,
                sections: BTreeMap::new(),
                problem: None,
            },
            FileEntry {
                path: "src/broken.rs".to_owned(),
                lines: 3,
                sections: BTreeMap::new(),
                problem: Some("src/broken.rs:2: section `x` starts and never ends".to_owned()),
            },
        ],
    }
}

fn links() -> CodeLinks {
    CodeLinks::new(index())
}

fn name(n: &str) -> ItemLink<'_> {
    ItemLink {
        name: n,
        ..ItemLink::default()
    }
}

fn find(l: &CodeLinks, link: &ItemLink<'_>) -> Result<String, String> {
    l.item(link)
        .map(|r| r.entry.path.clone())
        .map_err(|e| e.to_string())
}

#[test]
fn a_name_matches_by_its_end_and_a_reexport_is_one_item() {
    let l = links();
    for n in ["TapeWriter", "Writer", "writer::TapeWriter", "tape::Writer"] {
        assert_eq!(find(&l, &name(n)).unwrap(), "tape::TapeWriter", "{n}");
    }
    let r = l.item(&name("TapeWriter::resume_on")).unwrap();
    assert_eq!(
        r.key,
        "item tape::writer::TapeWriter::resume_on src/lib.rs:3"
    );
    assert_eq!(r.href, format!("{BLOB}src/lib.rs#L3-L9"));
    assert_eq!(r.text, "TapeWriter::resume_on");
}

#[test]
fn a_crate_prefix_narrows_the_match() {
    let l = links();
    assert_eq!(
        find(&l, &name("tape_fs::writer::resume")).unwrap(),
        "tape_fs::writer::resume"
    );
}

#[test]
fn a_whole_path_wins_over_paths_that_end_with_it() {
    let l = links();
    assert_eq!(
        find(&l, &name("Definitions")).unwrap_err(),
        "[`Definitions`]: several items have this name; write one of [`tape::Definitions`], [`tape::other::Definitions`]"
    );
    // each suggested form resolves
    assert_eq!(
        find(&l, &name("tape::Definitions")).unwrap(),
        "tape::Definitions"
    );
    assert_eq!(
        find(&l, &name("tape::other::Definitions")).unwrap(),
        "tape::other::Definitions"
    );
    assert_eq!(
        find(&l, &name("tape::open::Definitions")).unwrap(),
        "tape::Definitions"
    );
}

#[test]
fn an_ambiguous_name_says_how_to_write_each() {
    let l = links();
    assert!(
        find(&l, &name("resume"))
            .unwrap_err()
            .ends_with("write one of [`tape::chain::resume`], [`tape::writer::resume`], [`tape_fs::writer::resume`]")
    );
    assert!(
        find(&l, &name("level"))
            .unwrap_err()
            .ends_with("[`struct@tape::level`], [`fn@tape::level`]")
    );
    assert!(
        find(&l, &name("platform")).unwrap_err().ends_with(
            "[`cfg(unix)@tape::platform`], [`cfg(not(feature = \"std\"))@tape::platform`]"
        )
    );
}

#[test]
fn a_kind_an_arm_or_a_suffix_picks_one_item() {
    let l = links();
    let kind = |k, n| ItemLink {
        kind: Some(k),
        ..name(n)
    };
    assert_eq!(
        l.item(&kind("type", "level")).unwrap().entry.kind,
        Kind::Struct
    );
    let call = ItemLink {
        suffix: "()",
        ..name("level")
    };
    assert_eq!(l.item(&call).unwrap().entry.kind, Kind::Fn);
    assert_eq!(l.item(&call).unwrap().text, "level()");
    let arm = ItemLink {
        cfg: Some("cfg(not(feature=\"std\"))"),
        ..name("platform")
    };
    assert_eq!(l.item(&arm).unwrap().entry.lines[0], 120);
    assert_eq!(
        find(&l, &kind("thing", "level")).unwrap_err(),
        "[`thing@level`]: `thing@` is no kind; the kinds are const, constant, enum, field, fn, macro, method, static, struct, trait, tyalias, type, union, value, variant"
    );
    assert_eq!(
        find(&l, &name("Gone")).unwrap_err(),
        "[`Gone`]: no public item of the book's crates has this name"
    );
}

#[test]
fn a_file_or_section_resolves_or_says_why_not() {
    let l = links();
    let f = l.file("resume.rs", Some("call")).unwrap();
    assert_eq!(f.href, format!("{BLOB}tests/resume.rs#L3-L3"));
    assert_eq!(f.key, "file tests/resume.rs:call");
    assert_eq!(f.text, "resume.rs:call");
    let whole = l.file("tests/resume.rs", None).unwrap();
    assert_eq!(whole.href, format!("{BLOB}tests/resume.rs"));
    assert_eq!((whole.first, whole.last), (1, 4));
    let err = |n, s| l.file(n, s).unwrap_err().to_string();
    assert_eq!(
        err("gone.rs", None),
        "[`gone.rs`]: no file git tracks has this name"
    );
    assert_eq!(
        err("resume.rs", Some("cut")),
        "[`resume.rs:cut`]: tests/resume.rs marks no section `cut` (it marks: call)"
    );
    assert_eq!(
        err("broken.rs", None),
        "[`broken.rs`]: src/broken.rs:2: section `x` starts and never ends"
    );
    let bare = CodeLinks::new(Index {
        files: Vec::new(),
        ..index()
    });
    assert!(
        bare.file("x.rs", None)
            .unwrap_err()
            .to_string()
            .ends_with("(the index lists no files: vz refs ran outside a git checkout)")
    );
}

#[test]
fn intra_doc_links_open_the_items_lines_or_stay_plain() {
    let l = links();
    let at = &l.index.items[0];
    assert_eq!(
        l.doc_links(&at.doc, at),
        format!(
            "Reopen it: see [`Recovery`]({BLOB}src/lib.rs#L20-L26), [`Self::flush`]({BLOB}src/lib.rs#L40-L46) and `Gone`."
        )
    );
    let doc = "[a](crate::Recovery), [b][r], [c][], [https](https://x.org)\n\n[r]: Recovery\n\n```\n# hidden\nlet x = 1;\n```\n\n```text\n# kept\n```";
    assert_eq!(
        l.doc_links(doc, at),
        format!(
            "[a]({BLOB}src/lib.rs#L20-L26), [b]({BLOB}src/lib.rs#L20-L26), c, [https](https://x.org)\n\n\n```rust\nlet x = 1;\n```\n\n```text\n# kept\n```\n"
        )
    );
}

#[test]
fn long_docs_keep_what_fits_and_cut_after_a_sentence() {
    let para = "First `a. B` here. Second one is longer. Third.";
    assert_eq!(bounded(para, 1000), (para.to_owned(), false));
    assert_eq!(
        bounded(para, 40),
        ("First `a. B` here. Second one is longer.".to_owned(), true)
    );
    assert_eq!(bounded(para, 5), ("First `a. B` here.".to_owned(), true));
    let doc = "One.\n\n```\nlet a = 1;\n\nlet b = 2;\n```\n\nThree.";
    assert_eq!(bounded(doc, 1000), (doc.to_owned(), false));
    assert_eq!(bounded(doc, 10), ("One.".to_owned(), true));
    // characters, not bytes
    assert_eq!(bounded("Ünï.\n\nAb.", 7), ("Ünï.\n\nAb.".to_owned(), false));
}

#[test]
fn a_card_links_docs_rs_for_a_published_crate() {
    let mut ix = index();
    ix.crates.insert(
        "tape".to_owned(),
        CrateEntry {
            package: "tape-core".to_owned(),
            published: true,
        },
    );
    ix.items.extend([
        item("tape::Bits", Kind::Struct, 300),
        item("tape::Bits::word", Kind::Fn, 302),
        item("tape::Outcome", Kind::Enum, 310),
        item("tape::Outcome::Done", Kind::Variant, 312),
        item("tape::hidden::deep", Kind::Fn, 320),
        item("tape::fmt", Kind::Macro, 330),
    ]);
    let l = CodeLinks::new(ix);
    for (n, docs) in [
        ("tape::hidden::deep", "tape/hidden/fn.deep.html"),
        ("tape::Bits", "tape/struct.Bits.html"),
        ("tape::Bits::word", "tape/struct.Bits.html#method.word"),
        ("tape::Outcome::Done", "tape/enum.Outcome.html#variant.Done"),
        ("tape::fmt", "tape/macro.fmt.html"),
        (
            "TapeWriter::resume_on",
            "tape/writer/struct.TapeWriter.html#method.resume_on",
        ),
    ] {
        let r = l.item(&name(n)).unwrap();
        assert_eq!(
            l.docs_rs_url(&r.entry).unwrap(),
            format!("https://docs.rs/tape-core/latest/{docs}"),
            "{n}"
        );
        let card = l.item_card("id", &r, CardSizes::default(), &Plain).unwrap();
        assert!(card.contains(r#"">docs.rs</a></div>"#), "{card}");
    }
    let unpublished = links();
    let r = unpublished.item(&name("Recovery")).unwrap();
    assert_eq!(unpublished.docs_rs_url(&r.entry), None);
}

#[test]
fn an_item_card_holds_its_head_code_docs_and_links() {
    let l = links();
    let r = l.item(&name("resume_on")).unwrap();
    let card = l
        .item_card(
            "id",
            &r,
            CardSizes {
                lines_max: 16,
                doc_chars_max: 20,
            },
            &Plain,
        )
        .unwrap();
    assert_eq!(
        card,
        format!(
            concat!(
                r#"<div class="vz-card vz-card-code" id="id" role="tooltip">"#,
                r#"<div class="vz-card-head"><a href="{b}src/lib.rs#L3-L9">fn tape::writer::TapeWriter::resume_on</a></div>"#,
                r#"<div class="vz-card-src"><pre>```rust"#,
                "\npub fn resume_on()\n",
                r#"```</pre></div>"#,
                r#"<div class="vz-card-doc"><p>Reopen it: see [`Recovery`]({b}src/lib.rs#L20-L26), [`Self::flush`]({b}src/lib.rs#L40-L46) and `Gone`.</p></div>"#,
                r#"<div class="vz-card-where">src/lib.rs:3</div>"#,
                r#"<div class="vz-card-links"><a href="{b}src/lib.rs#L3-L9">Source on GitHub</a> · <a href="{b}src/lib.rs">File on GitHub</a></div>"#,
                "</div>"
            ),
            b = BLOB
        )
    );
}

#[test]
fn a_file_card_reads_its_lines_from_the_root() {
    let dir = std::env::temp_dir().join(format!("vz-links-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("tests")).unwrap();
    std::fs::write(
        dir.join("tests/resume.rs"),
        "fn x() {}\n// --8<-- [start:call]\nlet w = writer.resume_on(cut)?;\n// --8<-- [end:call]\n",
    )
    .unwrap();
    let l = links();
    let section = l.file("resume.rs", Some("call")).unwrap();
    let card = l.file_card("id", &section, &dir, 16, &Plain).unwrap();
    assert!(card.contains("tests/resume.rs, section call</a>"), "{card}");
    assert!(
        card.contains("```rust\nlet w = writer.resume_on(cut)?;\n```"),
        "{card}"
    );
    assert!(!card.contains("fn x()"), "{card}");
    assert!(card.contains(r#"<div class="vz-card-where">tests/resume.rs:3-3</div>"#));
    assert!(card.contains("Lines on GitHub</a> · <a"), "{card}");
    let whole = l.file("tests/resume.rs", None).unwrap();
    let card = l.file_card("id", &whole, &dir, 2, &Plain).unwrap();
    assert!(card.contains("2 more lines in the source"), "{card}");
    assert!(!card.contains("Lines on GitHub"), "{card}");
    let e = l
        .file_card("id", &whole, Path::new("/nonexistent"), 2, &Plain)
        .unwrap_err();
    assert!(e.to_string().starts_with("tests/resume.rs: "), "{e}");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn an_index_is_read_and_its_format_checked() {
    let dir = std::env::temp_dir().join(format!("vz-index-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("index.json");
    let err = |p: &Path| CodeLinks::read(p, "index.json").unwrap_err().to_string();
    assert_eq!(
        err(&path),
        "index.json is missing: run `vz prep` before the build"
    );
    std::fs::write(&path, r#"{"format": 1, "items": []}"#).unwrap();
    assert_eq!(
        err(&path),
        "index.json is not a code index of format 2: rebuild it with `vz prep`"
    );
    std::fs::write(&path, "{").unwrap();
    assert!(err(&path).starts_with("index.json: EOF"), "{}", err(&path));
    std::fs::write(
        &path,
        r#"{"format": 2, "commit": "c", "blob": "b", "items": [], "files": []}"#,
    )
    .unwrap();
    assert!(CodeLinks::read(&path, "index.json").is_ok());
    std::fs::remove_dir_all(&dir).unwrap();
}
