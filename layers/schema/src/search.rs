use std::collections::BTreeSet;
use std::str::FromStr;

use sdk::{framework_utils::args::ArgumentSplitter, models::*, Snowflake};

use crate::SnowflakeExt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Has {
    Image,
    Video,
    Audio,
    Link,
    Embed,
    File,
}

impl Has {
    pub fn as_str(self) -> &'static str {
        match self {
            Has::Image => "img",
            Has::Video => "vid",
            Has::Audio => "audio",
            Has::Link => "link",
            Has::Embed => "embed",
            Has::File => "file",
        }
    }

    pub fn as_mime(self) -> &'static str {
        match self {
            Has::Image => "image/",
            Has::Video => "video/",
            Has::Audio => "audio/",
            _ => unimplemented!(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum SearchTermKind {
    None,
    IsPinned,
    IsStarred,
    InThread,
    Id(Snowflake),
    Thread(Snowflake),
    Pinned(Snowflake),
    Has(Has),
    Query(String),
    Room(Snowflake),
    User(Snowflake),
    Before(Snowflake),
    After(Snowflake),
}

impl SearchTermKind {
    pub fn has(&self) -> Option<Has> {
        match self {
            SearchTermKind::Has(ty) => Some(*ty),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SearchTerm {
    pub kind: SearchTermKind,
    pub negated: bool,
}

impl SearchTerm {
    pub fn new(kind: SearchTermKind) -> Self {
        SearchTerm { kind, negated: false }
    }

    pub fn negate(mut self, negate: bool) -> Self {
        self.negated = negate;
        self
    }
}

bitflags::bitflags! {
    struct Existing: i8 {
        const ROOM = 1 << 0;
        const USER = 1 << 1;
        const BEFORE = 1 << 2;
        const AFTER = 1 << 3;
        const THREAD = 1 << 4;
        const ID = 1 << 5;
    }
}

/// A BTreeSet is used to ensure consistent ordering
pub fn parse_search_terms(q: &str) -> BTreeSet<SearchTerm> {
    let mut terms = BTreeSet::new();
    let args = ArgumentSplitter::split(q);

    let mut websearch = String::new();

    let mut existing = Existing::empty();

    for arg in args.arguments() {
        let inner = arg.inner_str();

        if inner.is_empty() || inner == "-" {
            continue;
        }

        let mut negated = false;

        let inner = match inner.strip_prefix('-') {
            _ if arg.is_quoted() => {
                // if this is a quoted argument, ignore the inner value
                // and look back to before it for a minus symbol
                let start = arg.outer().start;
                if let Some(prefix_start) = start.checked_sub(1) {
                    if &arg.orig()[prefix_start..start] == "-" {
                        negated = true;
                    }
                }

                inner
            }
            Some(inner) => {
                negated = true;

                inner
            }
            None => inner,
        };

        let mut term = SearchTerm {
            negated,
            kind: SearchTermKind::None,
        };

        if arg.is_quoted() {
            if negated {
                websearch.push_str("-");
            }
            websearch.push_str("\"");
            websearch.push_str(inner);
            websearch.push_str("\" ");
            continue;
        }

        let Some((category, value)) = inner.split_once(':') else {
            if negated {
                websearch.push_str("-");
            }
            websearch.push_str(inner);
            websearch.push_str(" ");
            continue;
        };

        #[allow(clippy::single_match)]
        match (category, value) {
            ("has", "image" | "img") => term.kind = SearchTermKind::Has(Has::Image),
            ("has", "video" | "vid") => term.kind = SearchTermKind::Has(Has::Video),
            ("has", "audio" | "sound") => term.kind = SearchTermKind::Has(Has::Audio),
            ("has", "link" | "url") => term.kind = SearchTermKind::Has(Has::Link),
            ("has", "embed") => term.kind = SearchTermKind::Has(Has::Embed),
            ("has", "file" | "attachment") => term.kind = SearchTermKind::Has(Has::File),
            ("in", "thread") => term.kind = SearchTermKind::InThread,
            ("is", "pinned") => term.kind = SearchTermKind::IsPinned,
            ("is", "starred") => term.kind = SearchTermKind::IsStarred,
            ("pinned", tag_id) => match Snowflake::from_str(tag_id) {
                Ok(id) => term.kind = SearchTermKind::Pinned(id),
                _ => {}
            },
            ("thread", thread_id) if !existing.contains(Existing::THREAD) => {
                match Snowflake::from_str(thread_id) {
                    Ok(id) => {
                        term.kind = SearchTermKind::Thread(id);
                        existing |= Existing::THREAD;
                    }
                    _ => {}
                }
            }
            ("in", room_id) if !existing.contains(Existing::ROOM) => match Snowflake::from_str(room_id) {
                Ok(id) => {
                    term.kind = SearchTermKind::Room(id);
                    existing |= Existing::ROOM;
                }
                _ => {}
            },
            ("from", user_id) if !existing.contains(Existing::USER) => match Snowflake::from_str(user_id) {
                Ok(id) => {
                    term.kind = SearchTermKind::User(id);
                    existing |= Existing::USER;
                }
                _ => {}
            },
            ("before", ts) if !existing.contains(Existing::BEFORE) => match Timestamp::parse(ts) {
                Some(ts) => {
                    term.kind = SearchTermKind::Before(Snowflake::at_ts(ts));
                    existing |= Existing::BEFORE;
                }
                None => match Snowflake::from_str(ts) {
                    Ok(ts) => {
                        term.kind = SearchTermKind::Before(ts);
                        existing |= Existing::BEFORE;
                    }
                    _ => {}
                },
            },
            ("after", ts) if !existing.contains(Existing::AFTER) => match Timestamp::parse(ts) {
                Some(ts) => {
                    term.kind = SearchTermKind::After(Snowflake::at_ts(ts));
                    existing |= Existing::AFTER;
                }
                None => match Snowflake::from_str(ts) {
                    Ok(ts) => {
                        term.kind = SearchTermKind::After(ts);
                        existing |= Existing::AFTER;
                    }
                    _ => {}
                },
            },
            ("id", id) if !existing.contains(Existing::ID) => match Snowflake::from_str(id) {
                Ok(id) => {
                    term.kind = SearchTermKind::Id(id);
                    existing |= Existing::ID;
                }
                _ => {}
            },
            _ => {
                if negated {
                    websearch.push_str("-");
                }
                websearch.push_str(inner);
                websearch.push_str(" ");
                continue;
            }
        }

        if term.kind != SearchTermKind::None {
            terms.insert(term);
        }
    }

    websearch.truncate(websearch.trim_end().len());
    if !websearch.is_empty() {
        terms.insert(SearchTerm {
            negated: false,
            kind: SearchTermKind::Query(websearch),
        });
    }

    terms
}
