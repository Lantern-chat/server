use super::*;

pub async fn message_create(
    state: &ServerState,
    id: Snowflake,
    party_id: Option<Snowflake>,
) -> Result<(), Error> {
    let db = state.db.read.get().await?;

    let row = db
        .query_one_cached_typed(
            || {
                use db::schema::*;

                Query::select()
                    .cols(&[
                        Messages::UserId,
                        Messages::RoomId,
                        Messages::Flags,
                        Messages::Content,
                    ])
                    .cols(&[PartyMember::Nickname])
                    .cols(&[Users::Username, Users::Discriminator])
                    .and_where(Messages::Id.equals(Var::of(Messages::Id)))
                    .from(
                        PartyMember::left_join(
                            Rooms::left_join(
                                Users::left_join_table::<Messages>()
                                    .on(Users::Id.equals(Messages::UserId)),
                            )
                            .on(Rooms::Id.equals(Messages::RoomId)),
                        )
                        .on(PartyMember::UserId.equals(Messages::UserId))
                        .on(PartyMember::PartyId.equals(Rooms::PartyId)),
                    )
            },
            &[&id],
        )
        .await?;

    Ok(())
}
