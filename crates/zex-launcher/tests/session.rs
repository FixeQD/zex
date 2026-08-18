use zex_launcher::apps::session_env;

#[test]
fn captures_the_process_environment() {
    let env = session_env();
    assert!(!env.is_empty());
    assert_eq!(env.get("PATH").is_some(), std::env::var_os("PATH").is_some());
}