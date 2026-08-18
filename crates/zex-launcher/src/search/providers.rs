//! Web search providers

/// A named web search backend reachable through `!trigger query`
pub struct Provider {
    pub trigger: &'static str,
    pub name: &'static str,
    template: &'static str,
}

/// Known providers, in display order
pub const PROVIDERS: &[Provider] = &[
    Provider {
        trigger: "ddg",
        name: "DuckDuckGo",
        template: "https://duckduckgo.com/?q={query}",
    },
    Provider {
        trigger: "gh",
        name: "GitHub",
        template: "https://github.com/search?q={query}&type=code",
    },
    Provider {
        trigger: "yt",
        name: "YouTube",
        template: "https://www.youtube.com/results?search_query={query}",
    },
    Provider {
        trigger: "wiki",
        name: "Wikipedia",
        template: "https://en.wikipedia.org/w/index.php?search={query}",
    },
];

impl Provider {
    pub fn template(&self) -> &'static str {
        self.template
    }
}

pub fn find(trigger: &str) -> Option<&'static Provider> {
    PROVIDERS
        .iter()
        .find(|provider| provider.trigger == trigger)
}

pub fn build_url(provider: &Provider, query: &str) -> String {
    provider
        .template
        .replace("{query}", &urlencoding::encode(query))
}

/// The fallback search URL, used when no trigger matches
pub fn default_url(query: &str) -> String {
    build_url(&PROVIDERS[0], query)
}
