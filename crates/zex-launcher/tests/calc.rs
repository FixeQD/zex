use zex_launcher::calc::evaluate;

#[test]
fn evaluates_arithmetic() {
    assert_eq!(evaluate("2+2"), Some("4".into()));
    assert_eq!(evaluate("10 / 4"), Some("2.5".into()));
}

#[test]
fn supports_functions_and_constants() {
    assert_eq!(evaluate("sqrt(16)"), Some("4".into()));
    assert_eq!(evaluate("pi"), Some("approx. 3.1415926536".into()));
}

#[test]
fn rejects_garbage() {
    assert_eq!(evaluate("not math"), None);
    assert_eq!(evaluate("firefox --new-window"), None);
}