use futures::future::Either;
use sdk::{api::commands::party::CreatePartyForm, models::*};
use smol_str::SmolStr;

use crate::prelude::*;

pub async fn create_party(
    state: ServerState,
    auth: Authorization,
    form: &Archived<CreatePartyForm>,
) -> Result<Party, Error> {
    if !schema::validation::validate_name(&form.name, state.config().shared.party_name_length.clone()) {
        return Err(Error::InvalidName);
    }

    let party_id = state.sf.gen();
    let room_id = state.sf.gen();

    let default_role = Role {
        id: party_id,
        party_id,
        avatar: None,
        desc: None,
        name: SmolStr::new_inline("@everyone"),
        permissions: Permissions::default(),
        color: None,
        position: 0,
        flags: RoleFlags::default(),
    };

    let mut party = Party {
        partial: PartialParty {
            id: party_id,
            name: SmolStr::from(&*form.name),
            description: form.description.as_deref().map(SmolStr::from),
        },
        flags: form.flags,
        avatar: None,
        banner: Nullable::Null,
        default_room: room_id,
        position: None,
        owner: auth.user_id(),
        roles: ThinVec::new(),
        emotes: ThinVec::new(),
        folders: ThinVec::new(),
    };

    let mut db = state.db.write.get().await?;
    let t = db.transaction().await?;

    // insert party first to avoid foreign key issues
    t.execute2(schema::sql! {
        INSERT INTO Party (Id, Name, Description, OwnerId, DefaultRoom) VALUES (
            #{&party.id           as Party::Id          },
            #{&party.name         as Party::Name        },
            #{&party.description  as Party::Description },
            #{&party.owner        as Party::OwnerId     },
            #{&party.default_room as Party::DefaultRoom }
        )
    })
    .await?;

    // TODO: Revisit this, it should probably go on top...
    let position = {
        #[rustfmt::skip]
        let row = t.query_one2(schema::sql! {
            SELECT MAX(PartyMembers.Position) AS @MaxPosition
            FROM PartyMembers WHERE PartyMembers.UserId = #{auth.user_id_ref() as PartyMembers::UserId}
        }).await?;

        match row.max_position::<Option<i16>>()? {
            Some(max_position) => max_position + 1,
            None => 0,
        }
    };

    party.position = Some(position);

    let [perm1, perm2] = default_role.permissions.to_i64();

    // NOTE: This is used to avoid lifetime issues
    futures::future::try_join3(
        t.execute2(schema::sql! {
            INSERT INTO PartyMembers (PartyId, UserId, Position) VALUES (
                #{&party.id          as PartyMembers::PartyId  },
                #{auth.user_id_ref() as PartyMembers::UserId   },
                #{&position          as PartyMembers::Position }
            )
        }),
        match form.flags.contains(PartyFlags::CLOSED) {
            // closed parties don't have roles, so skip the insert
            true => Either::Right(futures::future::ok(0)),
            false => Either::Left(t.execute2(schema::sql! {
                INSERT INTO Roles (Id, Name, PartyId, Permissions1, Permissions2) VALUES (
                    #{&default_role.id   as Roles::Id           },
                    #{&default_role.name as Roles::Name         },
                    #{&party.id          as Roles::PartyId      },
                    #{&perm1             as Roles::Permissions1 },
                    #{&perm2             as Roles::Permissions2 }
                )
            })),
        },
        t.execute2(schema::sql! {
            INSERT INTO Rooms (Id, PartyId, Name, Position, Flags) VALUES (
                #{&room_id              as Rooms::Id       },
                #{&party.id             as Rooms::PartyId  },
                // TODO: Get translations of this based on the provided language flags?
                #{&"general"            as Rooms::Name     },
                #{&0i16                 as Rooms::Position },
                #{&RoomFlags::DEFAULT   as Rooms::Flags    }
            )
        }),
    )
    .await?;

    t.commit().await?;

    party.roles.push(default_role);

    Ok(party)
}
