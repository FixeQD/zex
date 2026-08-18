use zex_launcher::clipboard::{Content, Entry, History, Settings};
use zex_launcher::engine::Matcher;

fn clip(text: &str) -> Entry {
    Entry::new(Content::Text(text.to_string()))
}

fn settings(limit: usize) -> Settings {
    Settings {
        limit,
        keep_passwords: false,
    }
}

#[test]
fn memory_only_history_captures_and_searches() {
    let mut history = History::open(None, settings(10)).unwrap();
    history.push(clip("alpha"));
    history.push(clip("beta"));

    assert_eq!(history.len(), 2);
    let all = history.browse(&Matcher::new(), "");
    assert_eq!(all[0].body(), "beta");
    assert_eq!(all[1].body(), "alpha");

    let hits = history.browse(&Matcher::new(), "alp");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].body(), "alpha");
}

#[test]
fn consecutive_duplicates_are_dropped() {
    let mut history = History::open(None, settings(10)).unwrap();
    assert!(history.push(clip("same")));
    assert!(!history.push(clip("same")));
    assert!(history.push(clip("other")));
    assert_eq!(history.len(), 2);
}

#[test]
fn newest_entry_stays_visible_after_duplicate_surge() {
    let mut history = History::open(None, settings(10)).unwrap();
    for n in 0..10 {
        history.push(clip(&format!("entry {n}")));
    }
    assert_eq!(history.browse(&Matcher::new(), "").first().unwrap().body(), "entry 9");
}

#[test]
fn ring_is_bounded_by_the_limit() {
    let mut history = History::open(None, settings(3)).unwrap();
    for n in 0..6 {
        history.push(clip(&format!("item {n}")));
    }
    assert_eq!(history.len(), 3);
    let all = history.browse(&Matcher::new(), "");
    assert_eq!(all[0].body(), "item 5");
    assert_eq!(all[2].body(), "item 3");
}

#[test]
fn persistence_survives_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("clipboard.sqlite");

    {
        let mut history = History::open(Some(&file), settings(10)).unwrap();
        history.push(clip("first"));
        history.push(clip("second"));
    }
    let reloaded = History::open(Some(&file), settings(10)).unwrap();
    assert_eq!(reloaded.len(), 2);
    let all = reloaded.browse(&Matcher::new(), "");
    assert_eq!(all[0].body(), "second");
    assert_eq!(all[1].body(), "first");
}

#[test]
fn persistence_respects_a_smaller_limit() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("clipboard.sqlite");

    {
        let mut history = History::open(Some(&file), settings(2)).unwrap();
        history.push(clip("one"));
        history.push(clip("two"));
        history.push(clip("three"));
    }
    let reloaded = History::open(Some(&file), settings(2)).unwrap();
    assert_eq!(reloaded.len(), 2);
    assert_eq!(reloaded.browse(&Matcher::new(), "").first().unwrap().body(), "three");
}

#[test]
fn clear_forgets_everything_even_on_disk() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("clipboard.sqlite");

    let mut history = History::open(Some(&file), settings(10)).unwrap();
    history.push(clip("gone"));
    history.clear();
    assert_eq!(history.len(), 0);

    let reloaded = History::open(Some(&file), settings(10)).unwrap();
    assert_eq!(reloaded.len(), 0);
}

#[test]
fn snippets_truncate_at_character_boundaries() {
    let long = "żółć".repeat(20);
    let entry = clip(&long);
    assert!(entry.snippet().len() < long.len());
    assert!(entry.snippet().ends_with("..."));

    let short = Entry::new(Content::Image {
        width: 4,
        height: 2,
        rgba: vec![0; 32],
    });
    assert_eq!(short.snippet(), "[image]");
    assert_eq!(short.content.kind_label(), "image");
}