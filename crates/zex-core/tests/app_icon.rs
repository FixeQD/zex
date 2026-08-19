//! App-icon resolution tests

use zex_core::app_icon::{best_match, jaccard_tokens, match_score, tokenize};

fn cand(id: &str, icon: &str) -> zex_core::app_icon::Candidate {
    zex_core::app_icon::Candidate {
        app_id: id.into(),
        name: id.into(),
        icon: Some(icon.into()),
    }
}

#[test]
fn tokenize_splits_and_lowercases() {
    assert_eq!(tokenize("Firefox.EXE"), ["firefox", "exe"]);
    assert_eq!(tokenize("org.gnome.Nautilus"), ["org", "gnome", "nautilus"]);
}

#[test]
fn jaccard_tokens_is_percent() {
    let a = tokenize("org.gnome.Nautilus");
    let b = tokenize("org.gnome.Console");
    assert_eq!(jaccard_tokens(&a, &b), 50);
    assert_eq!(jaccard_tokens(&a, &a), 100);
    assert_eq!(jaccard_tokens(&a, &[]), 0);
}

#[test]
fn scoring_prefers_exact() {
    let c = cand("org.gnome.Nautilus", "nautilus");
    assert_eq!(
        match_score("org.gnome.Nautilus", &tokenize("org.gnome.Nautilus"), &c),
        400
    );
    assert_eq!(match_score("Nautilus", &tokenize("Nautilus"), &c), 300);
}

#[test]
fn best_match_takes_highest_score() {
    let apps = [
        cand("org.gnome.Nautilus", "nautilus"),
        cand("org.gnome.Console", "console"),
    ];
    assert_eq!(
        best_match("org.gnome.Nautilus", &apps).unwrap().app_id,
        "org.gnome.Nautilus"
    );
    assert_eq!(
        best_match("nautilus", &apps).unwrap().app_id,
        "org.gnome.Nautilus"
    );
}

#[test]
fn partial_token_overlap_still_matches() {
    let apps = [
        cand("org.gnome.Console", "console"),
        cand("com.visualstudio.code", "code"),
    ];
    assert_eq!(
        best_match("org.gnome.Nautilus", &apps).unwrap().app_id,
        "org.gnome.Console"
    );
}

#[test]
fn no_overlap_matches_nothing() {
    let apps = [cand("org.gnome.Nautilus", "nautilus")];
    assert!(best_match("zzzzzz", &apps).is_none());
}
