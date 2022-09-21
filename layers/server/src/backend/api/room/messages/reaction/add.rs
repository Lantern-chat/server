use crate::{
    backend::{gateway::Event, util::encrypted_asset::encrypt_snowflake_opt},
    state::emoji::EmoteOrEmojiId,
    Authorization, Error, ServerState,
};
use futures::FutureExt;
use sdk::models::{events::UserReactionEvent, gateway::message::ServerMsg, *};

pub async fn add_reaction(
    state: ServerState,
    auth: Authorization,
    room_id: Snowflake,
    msg_id: Snowflake,
    emote: EmoteOrEmojiId,
) -> Result<(), Error> {
    let permissions =
        crate::backend::api::perm::get_cached_room_permissions(&state, auth.user_id, room_id).await?;

    if !permissions.contains(RoomPermissions::ADD_REACTIONS) {
        return Err(Error::Unauthorized);
    }

    let db = state.db.write.get().await?;

    use q::{Parameters, Params};

    let params = Params {
        user_id: auth.user_id,
        msg_id,
        room_id,
        emote_id: emote.emote(),
        emoji_id: emote.emoji(),
    };

    let params_p = &params.as_params();

    #[rustfmt::skip]
    let res = match (
        permissions.contains(RoomPermissions::USE_EXTERNAL_EMOTES),
        params.emoji_id.is_some(),
    ) {
        (false, false) => db.query_opt_cached_typed(|| q::query(false, false), params_p).boxed(),
        (false, true) => db.query_opt_cached_typed(|| q::query(false, true), params_p).boxed(),
        (true, false) => db.query_opt_cached_typed(|| q::query(true, false), params_p).boxed(),
        (true, true) => db.query_opt_cached_typed(|| q::query(true, true), params_p).boxed(),
    };

    let row = match res.await? {
        None => return Err(Error::Unauthorized),
        Some(row) => row,
    };

    use q::columns::Columns;

    if let Some(msg_id) = row.try_get(Columns::msg_id())? {
        let emote = match state.emoji.lookup(emote) {
            Some(emote) => emote,
            None => {
                log::error!("Error lookup up likely valid emote/emoji: {:?}", emote);
                return Ok(());
            }
        };

        let party_id = row.try_get(Columns::party_id())?;

        let event = ServerMsg::new_message_reaction_add(UserReactionEvent {
            emote,
            msg_id,
            room_id,
            party_id,
            user_id: auth.user_id,
            member: Some(Box::new(PartyMember {
                user: Some(User {
                    id: auth.user_id,
                    username: row.try_get(Columns::username())?,
                    discriminator: row.try_get(Columns::discriminator())?,
                    flags: UserFlags::from_bits_truncate_public(row.try_get(Columns::user_flags())?),
                    email: None,
                    preferences: None,
                    profile: match row.try_get(Columns::profile_bits())? {
                        None => Nullable::Null,
                        Some(bits) => Nullable::Some(UserProfile {
                            bits,
                            avatar: encrypt_snowflake_opt(&state, row.try_get(Columns::avatar_id())?).into(),
                            banner: Nullable::Undefined,
                            status: Nullable::Undefined,
                            bio: Nullable::Undefined,
                        }),
                    },
                }),
                nick: row.try_get(Columns::nickname())?,
                roles: row.try_get(Columns::role_ids())?,
                presence: None,
                flags: None,
            })),
        });

        match party_id {
            Some(party_id) => {
                state
                    .gateway
                    .broadcast_event(Event::new(event, Some(room_id))?, party_id)
                    .await;
            }
            None => unimplemented!(),
        }
    }

    Ok(())
}

mod q {
    use sdk::Snowflake;

    pub use schema::*;
    pub use thorn::*;

    thorn::params! {
        pub struct Params {
            pub user_id: Snowflake = Users::Id,
            pub msg_id: Snowflake = Messages::Id,
            pub emote_id: Option<Snowflake> = Emotes::Id,
            pub emoji_id: Option<i32> = Emojis::Id,
            pub room_id: Snowflake = Rooms::Id,
        }
    }

    thorn::tables! {
        pub struct Values {
            MsgId: Reactions::MsgId,
            EmoteId: Reactions::EmoteId,
            EmojiId: Reactions::EmojiId,
            UserIds: Reactions::UserIds,
            PartyId: Rooms::PartyId,
        }

        pub struct Inserted {
            MsgId: Values::MsgId,
        }

        pub struct ReactionEvent {
            MsgId: Inserted::MsgId,
            PartyId: Rooms::PartyId,
            Nickname: AggMembers::Nickname,
            Username: Users::Username,
            Discriminator: Users::Discriminator,
            UserFlags: Users::Flags,
            AvatarId: Profiles::AvatarId,
            ProfileBits: Profiles::Bits,
            RoleIds: AggMembers::RoleIds,
        }
    }

    thorn::decl_alias! {
        pub BaseProfile = Profiles,
        pub PartyProfile = Profiles
    }

    pub mod columns {
        use super::*;

        thorn::indexed_columns! {
            pub enum ValuesColumns {
                Values::MsgId,
            }

            pub enum Columns continue ValuesColumns {
                ReactionEvent::MsgId,
                ReactionEvent::PartyId,
                ReactionEvent::Nickname,
                ReactionEvent::Username,
                ReactionEvent::Discriminator,
                ReactionEvent::UserFlags,
                ReactionEvent::AvatarId,
                ReactionEvent::ProfileBits,
                ReactionEvent::RoleIds,
            }
        }
    }

    use columns::*;

    pub fn query(allow_external: bool, emoji: bool) -> impl AnyQuery {
        // first CTE, verified emote data and aggregates the values
        let mut values = Query::select()
            .exprs([
                Params::msg_id().alias_to(Values::MsgId),
                Params::emote_id().alias_to(Values::EmoteId),
                Params::emoji_id().alias_to(Values::EmojiId),
            ])
            .expr(Builtin::array(Params::user_id()).alias_to(Values::UserIds))
            // room_id may be unused, so just toss it here, will always be true
            .and_where(Params::room_id().is_not_null());

        // find emote and party_id
        values = if !emoji {
            values = match allow_external {
                true => values
                    .expr(PartyMember::PartyId.alias_to(Values::PartyId))
                    .from(
                        PartyMember::inner_join_table::<Emotes>()
                            .on(Emotes::PartyId.equals(PartyMember::PartyId)),
                    )
                    .and_where(PartyMember::UserId.equals(Params::user_id())),
                false => values
                    .expr(Rooms::PartyId.alias_to(Values::PartyId))
                    .from(Rooms::inner_join_table::<Emotes>().on(Emotes::PartyId.equals(Rooms::PartyId)))
                    .and_where(Rooms::Id.equals(Params::room_id())),
            };

            values.and_where(Emotes::Id.equals(Params::emote_id()))
        } else {
            values
                .expr(Rooms::PartyId.alias_to(Values::PartyId))
                .from_table::<Rooms>()
                .and_where(Rooms::Id.equals(Params::room_id()))
        };

        // second CTE, actually does the insertion
        let insert = Query::insert()
            .into::<Reactions>()
            .cols(&[
                Reactions::MsgId,
                Reactions::EmoteId,
                Reactions::EmojiId,
                Reactions::UserIds,
            ])
            .query(
                Query::select()
                    .cols(&[Values::MsgId, Values::EmoteId, Values::EmojiId, Values::UserIds])
                    .from_table::<Values>()
                    .as_value(),
            )
            .on_conflict(
                match emoji {
                    true => [Reactions::MsgId, Reactions::EmojiId],
                    false => [Reactions::MsgId, Reactions::EmoteId],
                },
                DoUpdate
                    .set(
                        Reactions::UserIds,
                        Builtin::array_append((Reactions::UserIds, Params::user_id())),
                    )
                    .and_where(Params::user_id().not_equals(Builtin::any((Reactions::UserIds,)))),
            )
            .returning(Reactions::MsgId.alias_to(Inserted::MsgId));

        // third CTE, based on if the insertion succeeds, fetch react object
        let fetch = Query::select()
            .expr(Inserted::MsgId.alias_to(ReactionEvent::MsgId))
            .expr(Values::PartyId.alias_to(ReactionEvent::PartyId))
            .exprs([
                AggMembers::Nickname.alias_to(ReactionEvent::Nickname),
                AggMembers::RoleIds.alias_to(ReactionEvent::RoleIds),
            ])
            .exprs([
                Users::Username.alias_to(ReactionEvent::Username),
                Users::Discriminator.alias_to(ReactionEvent::Discriminator),
                Users::Flags.alias_to(ReactionEvent::UserFlags),
            ])
            // ProfileColumns
            .expr(
                Builtin::coalesce((
                    PartyProfile::col(Profiles::AvatarId),
                    BaseProfile::col(Profiles::AvatarId),
                ))
                .alias_to(ReactionEvent::AvatarId),
            )
            .expr(
                Call::custom("lantern.combine_profile_bits")
                    .args((
                        BaseProfile::col(Profiles::Bits),
                        PartyProfile::col(Profiles::Bits),
                        PartyProfile::col(Profiles::AvatarId),
                    ))
                    .alias_to(ReactionEvent::ProfileBits),
            )
            .from(
                Values::inner_join_table::<Inserted>()
                    .on(Inserted::MsgId.equals(Values::MsgId))
                    .inner_join_table::<Users>()
                    .on(Users::Id.equals(Params::user_id()))
                    .left_join_table::<AggMembers>()
                    .on(AggMembers::UserId
                        .equals(Users::Id)
                        .and(AggMembers::PartyId.equals(Values::PartyId)))
                    .left_join_table::<BaseProfile>()
                    .on(BaseProfile::col(Profiles::UserId)
                        .equals(Params::user_id())
                        .and(BaseProfile::col(Profiles::PartyId).is_null()))
                    .left_join_table::<PartyProfile>()
                    .on(PartyProfile::col(Profiles::UserId)
                        .equals(Params::user_id())
                        .and(PartyProfile::col(Profiles::PartyId).equals(Values::PartyId))),
            );

        Query::select()
            .with(Values::as_query(values).exclude())
            .with(Inserted::as_query(insert).exclude())
            .with(ReactionEvent::as_query(fetch).exclude())
            .cols(ValuesColumns::default())
            .cols(Columns::default())
            .from(Values::left_join_table::<ReactionEvent>().on(true.lit()))
    }
}
