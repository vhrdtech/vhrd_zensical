use super::*;
use crate::cards::tests::Plain;

const TWO: &str = "[[term]]\nname = \"piece\"\nforms = [\"pieces\"]\nshort = \"One file.\"\nchapter = \"../pieces/\"\n\n[[term]]\nname = \"CRC\"\nlong = \"cyclic redundancy check\"\nshort = \"A <checksum>.\"\n";

fn glossary() -> BookGlossary {
    BookGlossary::new(Glossary::parse(TWO).unwrap(), "/glossary/")
}

#[test]
fn a_term_links_to_its_entry() {
    let r = glossary().find("Pieces").unwrap();
    assert_eq!(r.key, "term piece");
    assert_eq!(r.href, "/glossary/#term-piece");
    assert_eq!(
        glossary().find("chunk").unwrap_err().to_string(),
        "[[chunk]]: no such term in the glossary"
    );
}

#[test]
fn a_card_shows_the_name_long_form_and_definition() {
    let r = glossary().find("crc").unwrap();
    assert_eq!(
        BookGlossary::card("id", &r, &Plain).unwrap(),
        r#"<div class="vz-card vz-card-term" id="id" role="tooltip"><strong>CRC (cyclic redundancy check)</strong>: A &lt;checksum&gt;.</div>"#
    );
}

#[test]
fn the_marker_becomes_every_entry_in_order_of_name() {
    let lines = vec!["# Glossary".to_owned(), format!("  {PAGE_MARKER} ")];
    let out = glossary().expand(lines, &Plain, false).unwrap();
    assert_eq!(
        out,
        [
            "# Glossary",
            r#"<p class="vz-gloss" id="term-crc"><strong>CRC (cyclic redundancy check)</strong>: A &lt;checksum&gt;.</p>"#,
            "",
            r#"<p class="vz-gloss" id="term-piece"><strong>piece</strong>: One file. <a href="../pieces/">More</a></p>"#,
            "",
        ]
    );
}

#[test]
fn with_abbr_every_page_gets_the_long_forms() {
    let out = glossary()
        .expand(vec!["x".to_owned()], &Plain, true)
        .unwrap();
    assert_eq!(out, ["x", "", "*[CRC]: cyclic redundancy check"]);
}

#[test]
fn a_bad_file_names_the_file() {
    let dir = std::env::temp_dir().join(format!("vz-glossary-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("g.toml");
    std::fs::write(
        &path,
        "[[term]]\nname = \"a\"\nshort = \"b\"\nchapter = \"/abs/\"\n",
    )
    .unwrap();
    let e = BookGlossary::read(&path, "g.toml", "/glossary/").unwrap_err();
    assert!(
        e.to_string().starts_with("g.toml: term \"a\": chapter"),
        "{e}"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}
