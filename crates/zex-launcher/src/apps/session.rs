//! Session environment snapshot for launched processes

use std::collections::HashMap;
use std::sync::OnceLock;

static SESSION: OnceLock<HashMap<String, String>> = OnceLock::new();

/// The environment of the shell session
pub fn session_env() -> &'static HashMap<String, String> {
    SESSION.get_or_init(|| std::env::vars().collect())
}
