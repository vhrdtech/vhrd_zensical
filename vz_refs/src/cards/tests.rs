use super::*;

/// Renders Markdown as a paragraph or a code block, the way Python-Markdown wraps them
pub(crate) struct Plain;

impl Render for Plain {
    type Error = std::convert::Infallible;

    fn markdown(&self, text: &str) -> Result<String, Self::Error> {
        Ok(if text.starts_with("```") {
            format!("<pre>{}</pre>", escape(text))
        } else {
            text.split("\n\n")
                .map(|p| ["<p>", &escape(p), "</p>"].concat())
                .collect()
        })
    }
}

#[test]
fn a_card_id_is_a_slug_and_a_digest() {
    assert_eq!(
        card_id("item tape::writer::TapeWriter::resume_on src/lib.rs:3"),
        "vz-card-item-tape-writer-tapewriter-resume-on-src-lib-rs-81fde3"
    );
    assert_eq!(card_id("term CRC"), "vz-card-term-crc-4b363c");
    assert_eq!(card_id("  Ünïcode -- x!!"), "vz-card-n-code-x-3093b1");
}

#[test]
fn escape_writes_what_python_writes() {
    assert_eq!(
        escape(r#"<a href="x">'&'</a>"#),
        "&lt;a href=&quot;x&quot;&gt;&#x27;&amp;&#x27;&lt;/a&gt;"
    );
}

#[test]
fn inline_drops_a_single_paragraph() {
    assert_eq!(Plain.inline("a <b>").unwrap(), "a &lt;b&gt;");
    assert_eq!(Plain.inline("a\n\nb").unwrap(), "<p>a</p><p>b</p>");
}

#[test]
fn a_code_fence_is_longer_than_any_backtick_run_in_the_code() {
    assert_eq!(
        Plain.code("x", "rust").unwrap(),
        "<pre>```rust\nx\n```</pre>"
    );
    assert_eq!(
        Plain.code("a ```` b", "text").unwrap(),
        "<pre>`````text\na ```` b\n`````</pre>"
    );
}

#[test]
fn first_lines_counts_what_it_leaves_out() {
    assert_eq!(first_lines("a\nb\nc", 2), ("a\nb".to_owned(), 1));
    assert_eq!(first_lines("a", 2), ("a".to_owned(), 0));
    assert_eq!(more_lines(0), "");
    assert_eq!(
        more_lines(1),
        r#"<div class="vz-card-more">1 more line in the source</div>"#
    );
    assert_eq!(
        more_lines(2),
        r#"<div class="vz-card-more">2 more lines in the source</div>"#
    );
}

#[test]
fn the_stylesheet_styles_every_class_a_card_holds() {
    let css = crate::assets::CARD_CSS.text;
    for class in PART_CLASSES
        .iter()
        .chain(&["vz-card", "vz-gloss", "vz-cards", "vz-ref"])
    {
        assert!(css.contains(&format!(".{class}")), "no .{class}");
    }
    for kind in ["term", "code", "file"] {
        assert!(css.contains(&format!(".vz-{kind}")), "no .vz-{kind}");
    }
}
