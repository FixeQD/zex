use zex_launcher::search::providers::{build_url, default_url, find, PROVIDERS};

#[test]
fn finds_known_triggers() {
    assert_eq!(find("gh").unwrap().name, "GitHub");
    assert_eq!(find("ddg").unwrap().name, "DuckDuckGo");
    assert!(find("nope").is_none());
}

#[test]
fn templates_are_url_encoded() {
    let gh = find("gh").unwrap();
    assert_eq!(
        build_url(gh, "rust & cargo"),
        "https://github.com/search?q=rust%20%26%20cargo&type=code"
    );
}

#[test]
fn fallback_uses_the_first_provider() {
    assert_eq!(default_url("hello"), build_url(&PROVIDERS[0], "hello"));
}

#[test]
fn every_provider_has_a_trigger_and_template() {
    for provider in PROVIDERS {
        assert!(!provider.trigger.is_empty());
        assert!(provider.template().contains("{query}"));
    }
}