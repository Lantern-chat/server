use std::sync::Arc;

use arc_swap::ArcSwap;
use sdk::{models::EmoteOrEmoji, Snowflake};
use smol_str::SmolStr;
use std::collections::HashMap;

#[derive(Default, Debug, Clone)]
pub struct EmojiEntry {
    pub id: i32,
    pub emoji: SmolStr,
    pub description: Option<SmolStr>,
    pub category: Option<SmolStr>,
    pub aliases: Option<SmolStr>,
    pub tags: Option<SmolStr>,
}

#[derive(Debug, Default, Clone)]
struct EmojiMapInner {
    emoji_to_id: HashMap<SmolStr, i32>,
    id_to_entry: HashMap<i32, EmojiEntry>,
}

#[derive(Debug, Default)]
#[repr(transparent)]
pub struct EmojiMap(ArcSwap<EmojiMapInner>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmoteOrEmojiId {
    Emote(Snowflake),
    Emoji(i32),
}

impl EmoteOrEmojiId {
    pub const fn emote(self) -> Option<Snowflake> {
        match self {
            EmoteOrEmojiId::Emote(id) => Some(id),
            _ => None,
        }
    }

    pub const fn emoji(self) -> Option<i32> {
        match self {
            EmoteOrEmojiId::Emoji(id) => Some(id),
            _ => None,
        }
    }
}

impl EmojiMap {
    pub fn refresh(&self, entries: impl IntoIterator<Item = EmojiEntry>) {
        self.0.store(Arc::new(EmojiMapInner::new(entries)));
    }

    /// Maps a fully-qualified emoji to its database ID
    pub fn emoji_to_id(&self, emoji: &str) -> Option<i32> {
        self.0.load().emoji_to_id.get(emoji).copied()
    }

    /// Maps any possible emoji as its fully-qualified to database ID
    pub fn any_emoji_to_id(&self, emoji: &str) -> Option<i32> {
        emoji::find(emoji).and_then(|e| self.emoji_to_id(e))
    }

    pub fn id_to_emoji(&self, id: i32) -> Option<SmolStr> {
        self.with_entry(id, |e| e.emoji.clone())
    }

    pub fn with_entry<F, U>(&self, id: i32, f: F) -> Option<U>
    where
        F: FnOnce(&EmojiEntry) -> U,
    {
        self.0.load().id_to_entry.get(&id).map(f)
    }

    pub fn resolve(&self, e: EmoteOrEmoji) -> Option<EmoteOrEmojiId> {
        match e {
            EmoteOrEmoji::Emote { emote } => Some(EmoteOrEmojiId::Emote(emote)),
            EmoteOrEmoji::Emoji { emoji } => self.any_emoji_to_id(&emoji).map(EmoteOrEmojiId::Emoji),
        }
    }

    pub fn lookup(&self, e: EmoteOrEmojiId) -> Option<EmoteOrEmoji> {
        match e {
            EmoteOrEmojiId::Emoji(id) => self.id_to_emoji(id).map(|emoji| EmoteOrEmoji::Emoji { emoji }),
            EmoteOrEmojiId::Emote(id) => Some(EmoteOrEmoji::Emote { emote: id }),
        }
    }
}

impl EmojiMapInner {
    pub fn new(entries: impl IntoIterator<Item = EmojiEntry>) -> Self {
        let mut map = EmojiMapInner::default();

        for entry in entries {
            map.emoji_to_id.insert(entry.emoji.clone(), entry.id);
            map.id_to_entry.insert(entry.id, entry);
        }

        map
    }
}
