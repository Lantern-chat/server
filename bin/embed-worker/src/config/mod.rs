use hashbrown::{HashMap, HashSet};
use std::sync::Arc;

use regex::Regex;
use reqwest::header::HeaderName;

use self::header::DeHeaderValue;

pub mod header;
pub mod pattern;

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Missing site declaration for \"{0}\"")]
    MissingSite(String),

    #[error("Invalid Regex pattern in {0}")]
    InvalidRegex(&'static str),

    #[error("Invalid user agent for {0}")]
    InvalidUserAgent(String),

    #[error("Missing extractor field: extractors.{0}")]
    MissingExtractorField(&'static str),

    #[error("Invalid extractor field: extractors.{0}")]
    InvalidExtractorField(&'static str),
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ParsedConfig {
    #[serde(default = "defaults::default_redirects")]
    pub max_redirects: u32,

    /// Request timeout, in milliseconds
    #[serde(default = "defaults::default_timeout")]
    pub timeout: u64,

    #[serde(default = "defaults::default_resolve_media")]
    pub resolve_media: bool,

    #[serde(default)]
    pub prefixes: Vec<String>,

    #[serde(default)]
    pub allow_html: Vec<String>,

    #[serde(default)]
    pub skip_oembed: Vec<String>,

    #[serde(default)]
    pub sites: HashMap<String, Arc<Site>>,

    #[serde(default)]
    pub user_agents: HashMap<String, DeHeaderValue>,

    #[serde(default)]
    pub extractors: HashMap<String, HashMap<String, String>>,
}

#[rustfmt::skip]
mod defaults {
    pub const fn default_redirects() -> u32 { 2 }
    pub const fn default_timeout() -> u64 { 4000 }
    pub const fn default_resolve_media() -> bool { true }
}

#[derive(Default, Debug, Clone, serde::Deserialize)]
#[serde(default)]
pub struct Site {
    pub color: Option<u32>,
    pub pattern: Option<pattern::Pattern>,
    pub domains: HashSet<String>,
    pub user_agent: Option<String>,
    pub cookie: Option<DeHeaderValue>,
}

impl Site {
    pub fn matches(&self, domain: &str) -> bool {
        // Note: `contains` checks if the table is empty before hashing
        if self.domains.contains(domain) {
            return true;
        }

        match self.pattern {
            Some(ref p) => p.is_match(domain),
            None => false,
        }
    }

    pub fn add_headers(&self, config: &Config, mut req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if let Some(ref ua) = self.user_agent {
            if let Some(user_agent) = config.parsed.user_agents.get(ua) {
                println!("Using {user_agent:?} for User Agent");

                req = req.header(HeaderName::from_static("user-agent"), &**user_agent);
            }
        }

        if let Some(ref cookie) = self.cookie {
            req = req.header(HeaderName::from_static("cookie"), &**cookie);
        }

        req
    }
}

#[derive(Debug, Clone)]
pub enum DomainMatch {
    NoMatch,
    SimpleMatch,
    FullMatch(Arc<Site>),
}

impl DomainMatch {
    pub fn is_match(&self) -> bool {
        !matches!(self, DomainMatch::NoMatch)
    }
}

impl From<DomainMatch> for Option<Arc<Site>> {
    fn from(value: DomainMatch) -> Self {
        match value {
            DomainMatch::FullMatch(site) => Some(site),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct SitePatterns {
    pub patterns: Vec<Regex>,
    pub sites: Vec<Arc<Site>>,
}

impl SitePatterns {
    pub fn new(
        config: &ParsedConfig,
        raw: impl IntoIterator<Item = impl AsRef<str>>,
        error_name: &'static str,
    ) -> Result<SitePatterns, ConfigError> {
        let mut patterns = Vec::new();
        let mut sites = Vec::new();

        for pattern in raw {
            let pattern: &str = pattern.as_ref();

            if let Some(site_name) = pattern.strip_prefix('%') {
                let Some(site) = config.sites.get(site_name) else {
                    return Err(ConfigError::MissingSite(site_name.to_owned()));
                };

                sites.push(site.clone());
            } else {
                patterns.push(match Regex::new(pattern) {
                    Ok(re) => re,
                    Err(_) => return Err(ConfigError::InvalidRegex(error_name)),
                });
            }
        }

        Ok(SitePatterns { patterns, sites })
    }

    pub fn find(&self, domain: &str) -> DomainMatch {
        for site in &self.sites {
            if site.matches(domain) {
                return DomainMatch::FullMatch(site.clone());
            }
        }

        for pattern in &self.patterns {
            if pattern.is_match(domain) {
                return DomainMatch::SimpleMatch;
            }
        }

        DomainMatch::NoMatch
    }
}

#[derive(Debug)]
pub struct Config {
    pub parsed: ParsedConfig,

    pub allow_html: SitePatterns,
    pub skip_oembed: SitePatterns,
}

impl ParsedConfig {
    pub fn build(self) -> Result<Config, ConfigError> {
        Ok(Config {
            allow_html: SitePatterns::new(&self, self.allow_html.iter(), "allow_html")?,
            skip_oembed: SitePatterns::new(&self, self.skip_oembed.iter(), "skip_oembed")?,
            parsed: self,
        })
    }
}

impl Config {
    fn clean_domain<'a>(&self, mut domain: &'a str) -> &'a str {
        loop {
            let mut found = false;

            for prefix in &self.parsed.prefixes {
                if let Some(stripped) = domain.strip_prefix(prefix) {
                    domain = stripped;
                    found = true;
                }
            }

            if !found {
                break;
            }
        }

        domain
    }

    pub fn allow_html(&self, domain: &str) -> DomainMatch {
        self.allow_html.find(self.clean_domain(domain))
    }

    pub fn skip_oembed(&self, domain: &str) -> DomainMatch {
        self.skip_oembed.find(self.clean_domain(domain))
    }

    pub fn find_site(&self, domain: &str) -> Option<Arc<Site>> {
        let domain = self.clean_domain(domain);

        self.parsed.sites.values().find(|&site| site.matches(domain)).cloned()
    }
}
