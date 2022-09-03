use std::sync::Arc;

use arc_swap::ArcSwap;
use serde::{Deserialize, Serialize};
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
pub struct EmojiMap(ArcSwap<EmojiMapInner>);

impl EmojiMap {
    pub fn refresh(&self, entries: impl IntoIterator<Item = EmojiEntry>) {
        self.0.store(Arc::new(EmojiMapInner::new(entries)));
    }

    pub fn emoji_to_id(&self, emoji: &str) -> Option<i32> {
        self.0.load().emoji_to_id.get(emoji).copied()
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
