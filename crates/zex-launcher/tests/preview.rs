use zex_launcher::preview::plain_text;

#[test]
fn strips_basic_markdown() {
    let rendered = plain_text("Hello **bold** and `code`");
    assert_eq!(rendered, "Hello bold and code");
}

#[test]
fn keeps_paragraph_readability() {
    let rendered = plain_text("Line one\n\nLine two");
    assert!(rendered.contains("Line one"));
    assert!(rendered.contains("Line two"));
}

#[test]
fn handles_lists() {
    let rendered = plain_text("- one\n- two");
    assert_eq!(rendered, "one\ntwo");
}