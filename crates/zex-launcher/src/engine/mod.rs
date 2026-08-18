//! Result shaping: filtering, ranking and section grouping

pub mod filter;
pub mod section;

pub use filter::{Matcher, Scored, best_match, rank};
pub use section::{Section, organize};
