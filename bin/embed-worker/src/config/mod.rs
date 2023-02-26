use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use regex::Regex;

pub mod pattern;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ParsedConfig {
    #[serde(default)]
    pub prefixes: Vec<String>,

    #[serde(default)]
    pub allow_html: Vec<String>,

    #[serde(default)]
    pub skip_oembed: Vec<String>,

    #[serde(default)]
    pub sites: HashMap<String, Arc<Site>>,

    #[serde(default)]
    pub user_agents: HashMap<String, String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Site {
    #[serde(default)]
    pub color: Option<u32>,

    #[serde(default)]
    pub pattern: Option<pattern::Pattern>,

    #[serde(default)]
    pub domains: HashSet<String>,
}

impl Site {
    pub fn matches(&self, domain: &str) -> bool {
        if self.domains.contains(domain) {
            return true;
        }

        match self.pattern {
            Some(ref p) => p.is_match(domain),
            None => false,
        }
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

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Missing site declaration for \"{0}\"")]
    MissingSite(String),

    #[error("Invalid Regex pattern in {0}")]
    InvalidRegex(&'static str),
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
    pub user_agents: Vec<(Regex, String)>,
}

impl ParsedConfig {
    pub fn build(self) -> Result<Config, ConfigError> {
        Ok(Config {
            allow_html: SitePatterns::new(&self, self.allow_html.iter(), "allow_html")?,
            skip_oembed: SitePatterns::new(&self, self.skip_oembed.iter(), "skip_oembed")?,
            user_agents: {
                let mut user_agents = Vec::new();

                for (pattern, ua) in self.user_agents.iter() {
                    user_agents.push(match Regex::new(pattern) {
                        Ok(re) => (re, ua.clone()),
                        Err(_) => return Err(ConfigError::InvalidRegex("user_agents")),
                    })
                }

                user_agents
            },
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

    pub fn user_agent(&self, domain: &str) -> Option<&str> {
        for (re, ua) in &self.user_agents {
            if re.is_match(domain) {
                return Some(ua);
            }
        }

        None
    }

    pub fn allow_html(&self, domain: &str) -> DomainMatch {
        self.allow_html.find(self.clean_domain(domain))
    }

    pub fn skip_oembed(&self, domain: &str) -> DomainMatch {
        self.skip_oembed.find(self.clean_domain(domain))
    }

    pub fn find_site(&self, domain: &str) -> Option<Arc<Site>> {
        self.parsed.sites.values().find(|&site| site.matches(domain)).cloned()
    }
}
