use std::borrow::Cow;

use futures::FutureExt;
use sdk::{
    models::{Permission, RoomPermissions},
    Snowflake,
};
use smallvec::SmallVec;

use crate::{Authorization, Error, ServerState};

use md_utils::{is_spoilered, Span, SpanType};

pub async fn verify<'a>(
    t: &db::pool::Transaction<'_>,
    _state: &ServerState,
    auth: Authorization,
    room_id: Snowflake,
    perm: Permission,
    content: Cow<'a, str>,
    spans: &[Span],
) -> Result<Cow<'a, str>, Error> {
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

    let can_use_external = perm.contains(RoomPermissions::USE_EXTERNAL_EMOTES);

    use q::{Columns, Parameters, Params};

    let params = Params {
        kind_id: if can_use_external { auth.user_id } else { room_id },
        ids: &emote_ids,
    };

    #[rustfmt::skip]
    let rows = match can_use_external {
        true => t.query_cached_typed(q::match_all_usable_emotes, &params.as_params()).boxed().await,
        false => t.query_cached_typed(q::match_locally_usable_emotes, &params.as_params()).boxed().await,
    };

    let rows = rows?;

    let mut usable_emotes: SmallVec<[(&str, Snowflake); 16]> = SmallVec::new();

    for row in &rows {
        usable_emotes.push((
            row.try_get(Columns::name())?, //
            row.try_get(Columns::id())?,
        ));
    }

    let mut new_content = String::with_capacity(content.len());

    let mut last_end = 0;

    for &(span, old_name, old_id) in &emotes {
        new_content.push_str(&content[last_end..span.start() - 2]); // don't include "<:"

        use std::fmt::Write;

        // linear search is pretty fast for N<100
        match usable_emotes.iter().find(|e| e.1 == old_id) {
            Some(&(name, id)) => {
                write!(new_content, "<:{}:{}>", name, id).expect("write to string");
            }
            None => {
                write!(new_content, ":{}:", old_name).expect("write to string");
            }
        }

        last_end = span.end() + 1; // don't include ">"
    }

    new_content.push_str(&content[last_end..]);

    Ok(new_content.into())
}

mod q {
    use super::*;

    pub use schema::*;
    pub use thorn::*;

    thorn::params! {
        pub struct Params<'a> {
            pub kind_id: Snowflake = SNOWFLAKE,
            pub ids: &'a [Snowflake] = SNOWFLAKE_ARRAY,
        }
    }

    thorn::indexed_columns! {
        pub enum Columns {
            Emotes::Id,
            Emotes::Name,
        }
    }

    pub fn match_all_usable_emotes() -> impl AnyQuery {
        Query::select()
            .cols(Columns::default())
            .from(PartyMember::inner_join_table::<Emotes>().on(Emotes::PartyId.equals(PartyMember::PartyId)))
            .and_where(PartyMember::UserId.equals(Params::kind_id()))
            .and_where(Emotes::Id.equals(Builtin::any((Params::ids(),))))
    }

    pub fn match_locally_usable_emotes() -> impl AnyQuery {
        Query::select()
            .cols(Columns::default())
            .from(Rooms::inner_join_table::<Emotes>().on(Emotes::PartyId.equals(Rooms::PartyId)))
            .and_where(Rooms::Id.equals(Params::kind_id()))
            .and_where(Emotes::Id.equals(Builtin::any((Params::ids(),))))
    }
}
