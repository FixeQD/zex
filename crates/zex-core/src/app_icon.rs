//! App-id to icon resolution with a Jaccard fallback over a candidate catalog

pub const FALLBACK_ICON: &str = "application-x-executable-symbolic";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub app_id: String,
    pub name: String,
    pub icon: Option<String>,
}

/// Lowercased tokens split on `._-`
pub fn tokenize(s: &str) -> Vec<String> {
    s.to_lowercase()
        .split(['.', '_', '-'])
        .filter(|t| !t.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Jaccard similarity of two token sets, in percent
pub fn jaccard_tokens(a: &[String], b: &[String]) -> u32 {
    if a.is_empty() && b.is_empty() {
        return 0;
    }
    let inter = a.iter().filter(|t| b.contains(t)).count();
    let union = a.len() + b.len() - inter;
    if union == 0 {
        return 0;
    }
    ((inter as f64 / union as f64) * 100.0) as u32
}

/// Match strength of a candidate against a query app-id:
/// exact 400, substring 300, name 200, token Jaccard scaled to 100
pub fn match_score(query: &str, query_tokens: &[String], candidate: &Candidate) -> u32 {
    let exact_or_substring = if query == candidate.app_id {
        400
    } else if candidate.app_id.contains(query) {
        300
    } else {
        0
    };
    let name = if query == candidate.name.to_lowercase() {
        200
    } else {
        0
    };
    let id_tokens = tokenize(&candidate.app_id);
    let name_tokens = tokenize(&candidate.name);
    let jaccard =
        jaccard_tokens(query_tokens, &id_tokens).max(jaccard_tokens(query_tokens, &name_tokens));
    exact_or_substring.max(name).max(jaccard)
}

/// Best catalog match for a query, if any token overlap exists
pub fn best_match<'a>(app_id: &str, candidates: &'a [Candidate]) -> Option<&'a Candidate> {
    let query_tokens = tokenize(app_id);
    candidates
        .iter()
        .map(|c| (match_score(app_id, &query_tokens, c), c))
        .max_by_key(|(score, _)| *score)
        .and_then(|(score, c)| (score > 0).then_some(c))
}

/// Resolved icon for a query: exact candidate icon, else the best match's icon
pub fn resolve_icon(app_id: &str, candidates: &[Candidate]) -> Option<String> {
    candidates
        .iter()
        .find(|c| c.app_id == app_id)
        .or_else(|| best_match(app_id, candidates))
        .and_then(|c| c.icon.clone())
}
