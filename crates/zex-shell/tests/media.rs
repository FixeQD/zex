//! Media widget helpers: percent-decoding of art URLs

use zex_shell::bar::widgets::media::percent_decode;

#[test]
fn percent_decode_resolves_escapes() {
    assert_eq!(percent_decode("//a%20b.jpg").as_deref(), Some("//a b.jpg"));
    assert_eq!(percent_decode("plain-path").as_deref(), Some("plain-path"));
    assert!(percent_decode("bad%zz").is_none());
}

#[test]
fn percent_decode_handles_utf8_and_edge_cases() {
    // Multi-byte UTF-8 escape sequences decode back into the original characters
    assert_eq!(percent_decode("caf%C3%A9.png").as_deref(), Some("café.png"));
    // An escape that produces invalid UTF-8 fails cleanly
    assert!(percent_decode("%FF%00%00").is_none());
    // A trailing percent with no room for two hex digits passes through unchanged
    assert_eq!(percent_decode("100%").as_deref(), Some("100%"));
    assert_eq!(percent_decode("%").as_deref(), Some("%"));
    assert_eq!(percent_decode("a%2").as_deref(), Some("a%2"));
    // Empty input stays empty
    assert_eq!(percent_decode("").as_deref(), Some(""));
}

#[test]
fn percent_decode_is_hex_insensitive_and_only_blows_up_on_bad_pairs() {
    assert_eq!(percent_decode("a%2fb").as_deref(), Some("a/b"));
    assert_eq!(percent_decode("a%2Fb").as_deref(), Some("a/b"));
    assert_eq!(percent_decode("%0a").as_deref(), Some("\n"));
    // A percent with no room for a full hex pair is left as-is, not rejected
    assert_eq!(percent_decode("%2").as_deref(), Some("%2"));
}
