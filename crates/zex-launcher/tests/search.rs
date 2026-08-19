use zex_launcher::search::Intent;
use zex_launcher::search::detect;

#[test]
fn plain_text_is_browsing() {
    assert_eq!(detect("firefox"), Intent::Browse("firefox".into()));
    assert_eq!(detect("  firefox  "), Intent::Browse("firefox".into()));
}

#[test]
fn leading_arrow_runs_a_command() {
    assert_eq!(detect(">ls -la"), Intent::Run("ls -la".into()));
    assert_eq!(detect(">  uptime"), Intent::Run("uptime".into()));
}

#[test]
fn bang_trigger_is_web_search() {
    assert_eq!(
        detect("!gh rust combinators"),
        Intent::Web {
            trigger: "gh".into(),
            query: "rust combinators".into()
        }
    );
    assert_eq!(
        detect("!wiki berlin"),
        Intent::Web {
            trigger: "wiki".into(),
            query: "berlin".into()
        }
    );
}

#[test]
fn lone_bang_is_not_web_search() {
    assert_eq!(detect("!"), Intent::Browse("!".into()));
    assert_eq!(detect("!gh"), Intent::Browse("!gh".into()));
}

#[test]
fn arithmetic_is_calculated() {
    assert_eq!(
        detect("2+2"),
        Intent::Math {
            expression: "2+2".into(),
            answer: Some("4".into())
        }
    );
    assert_eq!(
        detect("17 * 3 - 20%"),
        Intent::Math {
            expression: "17 * 3 - 20%".into(),
            answer: Some("50.8".into())
        }
    );
}

#[test]
fn prose_with_numbers_is_not_math() {
    assert_eq!(
        detect("top 10 movies"),
        Intent::Browse("top 10 movies".into())
    );
    assert_eq!(detect("ip address"), Intent::Browse("ip address".into()));
}
