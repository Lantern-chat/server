use hashbrown::HashMap;
use heck::ToTitleCase;
use sdk::models::SmolStr;

#[derive(Debug, serde::Deserialize)]
#[serde(untagged)]
pub enum E621Result<T> {
    Failure(Failure),
    Success(T),
}

#[derive(Debug, serde::Deserialize)]
pub struct Failure {
    pub success: monostate::MustBe!(false),
    //pub message: String,
    //pub code: String,
}

/// Avoid allocating a whole `Vec` just for a single post,
/// which will only return an array of 0 or 1 elements.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, serde::Deserialize)]
#[serde(untagged)]
pub enum SinglePost {
    Found { posts: [Post; 1] },
    NotFound { posts: [Post; 0] },
}

#[derive(Debug, serde::Deserialize)]
pub struct Posts {
    pub posts: Vec<Post>,
}

#[derive(Debug, serde::Deserialize)]
pub struct Post {
    pub file: File,
    #[serde(default)]
    pub preview: Option<File>,
    #[serde(default)]
    pub sample: Option<Sample>,
    pub rating: Rating,
    pub description: SmolStr,
    pub tags: Tags,
}

// matches file, preview, and sample
#[derive(Debug, serde::Deserialize)]
pub struct File {
    pub width: u32,
    pub height: u32,

    #[serde(default)]
    pub size: Option<usize>,

    #[serde(default)]
    pub url: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct Sample {
    #[serde(flatten)]
    pub file: File,

    #[serde(default)]
    pub alternates: HashMap<SmolStr, Alternate>,
}

#[derive(Debug, serde::Deserialize)]
pub struct Alternate {
    #[serde(rename = "type")]
    pub kind: SmolStr,
    pub height: u32,
    pub width: u32,

    #[serde(default)]
    pub urls: Vec<Option<String>>,
}

#[derive(Debug, serde::Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rating {
    #[serde(rename = "s")]
    Safe,
    #[serde(rename = "q")]
    Questionable,
    #[serde(rename = "e")]
    Explicit,
}

#[derive(Debug, serde::Deserialize)]
pub struct Tags {
    #[serde(default)]
    pub general: Vec<SmolStr>,
    #[serde(default)]
    pub artist: Vec<SmolStr>,
    #[serde(default)]
    pub character: Vec<SmolStr>,
    #[serde(default)]
    pub species: Vec<SmolStr>,
    #[serde(default)]
    pub copyright: Vec<SmolStr>,
}

use std::fmt::{self, Write};

const MORE: &SmolStr = &SmolStr::new_inline("more");

impl Post {
    pub fn generate_author(&mut self) -> Result<Option<String>, fmt::Error> {
        self.tags.artist.retain(|tag| {
            // meta-artist tags we don't want to display
            !matches!(tag.as_str(), "avoid_posting" | "conditional_dnp")
        });

        if self.tags.artist.is_empty() {
            return Ok(None);
        }

        let mut author = String::new();
        let mut rest_storage = SmolStr::default();

        crate::util::format_list(
            &mut author,
            std::iter::empty().chain(self.tags.artist.iter().take(4)).chain(
                // if there are more than 4 artists listed, add "N more" to list
                match self.tags.artist.len().checked_sub(4) {
                    Some(remaining) if remaining > 0 => {
                        rest_storage = smol_str::format_smolstr!("{remaining} more");
                        Some(&rest_storage)
                    }
                    _ => None,
                },
            ),
        )?;

        Ok(Some(author.replace('_', " ")))
    }

    pub fn generate_title(&self) -> Result<String, fmt::Error> {
        let mut title = String::new();

        title += match self.rating {
            Rating::Safe => "safe, ",
            Rating::Questionable => "questionable, ",
            Rating::Explicit => "explicit, ",
        };

        crate::util::format_list(
            &mut title,
            std::iter::empty()
                .chain(self.tags.character.iter().take(5))
                .chain(self.tags.general.iter().take(4))
                .chain(self.tags.species.iter().take(3))
                .chain((self.tags.species.len() > 3).then_some(MORE))
                .map(|name| name.split("_(").next().unwrap()),
        )?;

        if !self.tags.copyright.is_empty() {
            if !title.is_empty() {
                title += ". ";
            }

            let list = self
                .tags
                .copyright
                .iter()
                .map(|c| heck::AsTitleCase(c.as_str()))
                .take(4);

            title += "(© ";
            crate::util::format_list(&mut title, list)?;
            title += ")";
        }

        Ok(title.replace('_', " "))
    }
}
