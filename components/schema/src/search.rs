use std::str::FromStr;

use sdk::{framework_utils::args::ArgumentSplitter, models::*, Snowflake};

use crate::SnowflakeExt;

pub use vec_collections::VecSet;

pub type SearchTerms = VecSet<[SearchTerm; 8]>;

#[derive(Debug, thiserror::Error)]
pub enum SearchError {
    #[error("Empty Search")]
    Empty,

    #[error("Invalid Term")]
    InvalidTerm,

    #[error("Invalid Prefix")]
    InvalidPrefix,

    #[error("Invalid Regex")]
    InvalidRegex,
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Sort {
    Relevant,
    Ascending,
    #[default]
    Descending,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Has {
    Text,
    Code,
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
            Has::Text => "text",
            Has::Code => "code",
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
    Sort(Sort),
    Id(Snowflake),
    Parent(Snowflake),
    Pinned(Snowflake),
    Has(Has),
    Query(SmolStr),
    Room(Snowflake),
    User(Snowflake),
    Before(Snowflake),
    After(Snowflake),
    Prefix(SmolStr),
    Regex(SmolStr),
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
        const PREFIX = 1 << 6;
        const SORT = 1 << 7;
    }
}

/// A BTreeSet is used to ensure consistent ordering
pub fn parse_search_terms(q: &str) -> Result<SearchTerms, SearchError> {
    let mut terms = SearchTerms::empty();
    let args = ArgumentSplitter::split(q);

    let mut websearch = String::new();

    let mut existing = Existing::empty();
    let mut num_pins = 0;

    let mut args_iter = args.arguments().iter().peekable();

    while let Some(arg) = args_iter.next() {
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

        let mut matched_next = false;

        let maybe_category_value = if arg.is_quoted() {
            if arg.is_quoted_with(('`', '`')) {
                term.kind = SearchTermKind::Regex(normalize_regex_syntax(inner)?.into());
                terms.insert(term);
                continue;
            }

            if negated {
                websearch.push_str("-");
            }
            websearch.push_str("\"");
            websearch.push_str(inner);
            websearch.push_str("\" ");
            continue;
        } else {
            let mut res = None;
            if let Some(inner) = inner.strip_suffix(':') {
                if let Some(next) = args_iter.peek() {
                    if arg.outer().end == next.outer().start {
                        res = Some((inner, next.inner_str()));
                        matched_next = true;
                    }
                }
            }

            res.or_else(|| inner.split_once(':'))
        };

        let Some((category, value)) = maybe_category_value else {
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
            ("has", "text" | "txt" | "message" | "msg" | "content") => term.kind = SearchTermKind::Has(Has::Text),
            ("has", "code") => term.kind = SearchTermKind::Has(Has::Code),
            ("in", "thread") => term.kind = SearchTermKind::InThread,
            ("is", "pinned") => term.kind = SearchTermKind::IsPinned,
            ("is", "starred") => term.kind = SearchTermKind::IsStarred,
            ("sort" | "order", order) if !existing.contains(Existing::SORT) => {
                let order = match order {
                    "asc" | "ascend" | "ascending" | "reverse" | "old" | "oldest" => Sort::Ascending,
                    "desc" | "descend" | "descending" | "new" | "newest" => Sort::Descending,
                    "rel" | "relevance" | "relevant" | "rank" | "score" => Sort::Relevant,
                    _ => continue,
                };

                term.kind = SearchTermKind::Sort(order);
                existing |= Existing::SORT;
            }
            ("pinned" | "pin", tag_id) => match Snowflake::from_str(tag_id) {
                Ok(id) if num_pins < 10 => {
                    num_pins += 1;
                    term.kind = SearchTermKind::Pinned(id)
                }
                _ => {}
            },
            ("thread" | "parent", thread_id) if !existing.contains(Existing::THREAD) => {
                match Snowflake::from_str(thread_id) {
                    Ok(id) => {
                        term.kind = SearchTermKind::Parent(id);
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
            ("starts_with" | "prefix", value) if !existing.contains(Existing::PREFIX) => {
                let Some(mut value) = normalize_similar(value) else {
                    return Err(SearchError::InvalidPrefix);
                };
                value.push('%');
                term.kind = SearchTermKind::Prefix(value.into());
                existing |= Existing::PREFIX;
            }
            _ => {
                if negated {
                    websearch.push_str("-");
                }
                websearch.push_str(inner);
                websearch.push_str(" ");
                continue;
            }
        }

        if matched_next {
            args_iter.next();
        }

        if term.kind != SearchTermKind::None {
            terms.insert(term);
        }
    }

    websearch.truncate(websearch.trim_end().len());

    // silently ignore non-text searches
    if !websearch.is_empty() && websearch.contains(char::is_alphanumeric) {
        terms.insert(SearchTerm {
            negated: false,
            kind: SearchTermKind::Query(websearch.into()),
        });
    }

    if terms.is_empty() {
        return Err(SearchError::Empty);
    }

    Ok(terms)
}

fn normalize_similar(mut s: &str) -> Option<String> {
    s = s.trim_matches('^');

    if !s.starts_with(|c: char| c.is_alphanumeric() || !matches!(c, '\\' | '[' | '(' | '{' | '.')) {
        return None;
    }

    let s = s.to_lowercase();

    let mut escaped = false;

    let mut out = String::new();
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\\' || escaped {
            escaped ^= true;
            out.push(c);
            continue;
        }

        match c {
            '.' => match chars.peek() {
                Some('*') => {
                    _ = chars.next(); // consume
                    out.push('%');
                }
                _ => out.push('_'),
            },
            '_' | '%' => {
                out.push('\\');
                out.push(c);
            }
            _ => out.push(c),
        }
    }

    Some(out)
}

#[cfg(test)]
mod tests {
    use super::{normalize_regex_syntax, normalize_similar, parse_search_terms};

    #[test]
    fn test_similar() {
        println!("{:?}", normalize_similar("!echo"));

        println!("{:?}", parse_search_terms("-prefix:\"!echo \""));

        println!("{:?}", normalize_regex_syntax("(tes[ts])"));
    }
}

fn normalize_regex_syntax(re: &str) -> Result<String, SearchError> {
    use regex_syntax::ast::{parse::ParserBuilder, visit, Ast, Visitor};

    struct ReAstVisitor;

    impl Visitor for ReAstVisitor {
        type Output = ();
        type Err = SearchError;

        fn finish(self) -> Result<(), Self::Err> {
            Ok(())
        }

        //fn visit_pre(&mut self, _ast: &Ast) -> Result<(), Self::Err> {
        //    // TODO
        //    Ok(())
        //}
    }

    //if re.starts_with('^') {
    //    if let Some(mut prefix) = normalize_similar(re) {
    //        prefix.push('%');
    //    }
    //}

    let Ok(ast) = ParserBuilder::new().nest_limit(6).build().parse(re) else {
        return Err(SearchError::InvalidRegex);
    };

    //visit(&ast, ReAstVisitor)?;

    Ok(re.to_owned())
}
