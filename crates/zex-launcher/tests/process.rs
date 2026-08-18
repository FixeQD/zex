use zex_launcher::process::{fire, in_terminal, shell};

#[test]
fn fires_a_simple_command() {
    fire("true", &[]).unwrap();
    shell("true").unwrap();
}

#[test]
fn empty_input_is_refused() {
    assert!(fire("", &[]).is_err());
    assert!(shell("   ").is_err());
}

#[test]
fn missing_binary_is_an_error() {
    assert!(fire("zex-no-such-binary-xyz", &[]).is_err());
}

#[test]
fn terminal_template_wraps_the_command() {
    in_terminal("echo %command%", "hello").unwrap();
    in_terminal("echo", "hello").unwrap();
}