use std::borrow::Cow;

use sdk::{models::Permissions, Snowflake};
use smallvec::SmallVec;

use crate::{Authorization, Error, ServerState};

use md_utils::SpanType;

pub async fn verify<'a>(
    t: &db::pool::Transaction<'_>,
    _state: &ServerState,
    auth: Authorization,
    room_id: Snowflake,
    perms: Permissions,
    content: Cow<'a, str>,
) -> Result<Cow<'a, str>, Error> {
    let spans = md_utils::scan_markdown(&content);

    let mut emotes = Vec::new();

    for span in spans {
        if span.kind() == SpanType::CustomEmote {
            if let Some((name, id)) = content[span.range()].split_once(':') {
                if let Ok(id) = id.parse::<Snowflake>() {
                    emotes.push((span, name, id));
                }
            }
        }
    }

    if emotes.is_empty() {
        return Ok(content);
    }

    let mut emote_ids: SmallVec<[Snowflake; 8]> = emotes.iter().map(|&(_, _, id)| id).collect();

    emote_ids.sort_unstable();
    emote_ids.dedup();

    let emote_ids = emote_ids.as_slice();

    #[rustfmt::skip]
    let rows = t.query2(schema::sql! {
        SELECT
            Emotes.Id AS @_,
            Emotes.Name AS @_
        FROM Emotes INNER JOIN match perms.contains(Permissions::USE_EXTERNAL_EMOTES) {
            true => {
                PartyMembers ON PartyMembers.PartyId = Emotes.PartyId
                WHERE PartyMembers.UserId = #{&auth.user_id as Users::Id}
            },
            false => {
                LiveRooms AS Rooms ON Rooms.PartyId = Emotes.PartyId
                WHERE Rooms.Id = #{&room_id as Rooms::Id}
            },
        }
        AND Emotes.Id = ANY(#{&emote_ids as SNOWFLAKE_ARRAY})
    }).await?;

    let mut usable_emotes: SmallVec<[(&str, Snowflake); 16]> = SmallVec::new();

    for row in &rows {
        usable_emotes.push((row.emotes_name()?, row.emotes_id()?));
    }

    let mut new_content = String::with_capacity(content.len());

    let mut last_end = 0;

    for &(span, old_name, old_id) in &emotes {
        new_content.push_str(&content[last_end..span.start() - 2]); // don't include "<:"

        use std::fmt::Write;

        // linear search is pretty fast for N<100
        match usable_emotes.iter().find(|e| e.1 == old_id) {
            Some(&(name, id)) => {
                write!(new_content, "<:{name}:{id}>").expect("write to string");
            }
            None => {
                write!(new_content, ":{old_name}:").expect("write to string");
            }
        }

        last_end = span.end() + 1; // don't include ">"
    }

    new_content.push_str(&content[last_end..]);

    Ok(new_content.into())
}
