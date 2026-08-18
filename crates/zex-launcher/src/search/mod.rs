//! Query analysis and web search providers

pub mod providers;

pub use providers::{PROVIDERS, Provider, build_url, default_url, find};

use crate::calc::evaluate;
use regex::Regex;
use std::sync::OnceLock;

#[derive(Clone, Debug, PartialEq)]
pub enum Intent {
    Browse(String),
    Run(String),
    Web {
        trigger: String,
        query: String,
    },
    Math {
        expression: String,
        answer: Option<String>,
    },
}

fn math_pattern() -> &'static Regex {
    static MATH: OnceLock<Regex> = OnceLock::new();
    MATH.get_or_init(|| Regex::new(r"^[\d+\-*/^().,%\s]+$").expect("valid math regex"))
}

fn looks_like_math(expr: &str) -> bool {
    let has_digit = expr.chars().any(|c| c.is_ascii_digit());
    let has_op = expr
        .chars()
        .any(|c| matches!(c, '+' | '-' | '*' | '/' | '^' | '%'));
    has_digit && has_op && math_pattern().is_match(expr)
}

/// Classify a raw query
pub fn detect(raw: &str) -> Intent {
    let trimmed = raw.trim();
    if let Some(command) = trimmed.strip_prefix('>') {
        return Intent::Run(command.trim().to_string());
    }
    if let Some(rest) = trimmed.strip_prefix('!') {
        let mut words = rest.split_whitespace();
        if let (Some(trigger), Some(word)) = (words.next(), words.next()) {
            let query = std::iter::once(word)
                .chain(words)
                .collect::<Vec<_>>()
                .join(" ");
            return Intent::Web {
                trigger: trigger.to_string(),
                query,
            };
        }
    }
    if looks_like_math(trimmed) {
        return Intent::Math {
            expression: trimmed.to_string(),
            answer: evaluate(trimmed),
        };
    }
    Intent::Browse(trimmed.to_string())
}
