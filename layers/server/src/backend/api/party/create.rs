use std::time::SystemTime;

use schema::SnowflakeExt;

use sdk::models::*;
use smol_str::SmolStr;

use crate::{Authorization, Error, ServerState};

#[derive(Debug, Clone, Deserialize)]
pub struct PartyCreateForm {
    name: SmolStr,

    #[serde(default)]
    description: Option<SmolStr>,

    #[serde(default)]
    security: SecurityFlags,
}

pub async fn create_party(state: ServerState, auth: Authorization, form: PartyCreateForm) -> Result<Party, Error> {
    if !state.config().party.partyname_len.contains(&form.name.len()) {
        return Err(Error::InvalidName);
    }

    let party_id = Snowflake::now();
    let room_id = Snowflake::now();

    let default_role = Role {
        id: party_id,
        party_id,
        avatar: None,
        name: SmolStr::new_inline("@everyone"),
        permissions: Permissions::default(),
        color: None,
        position: 0,
        flags: RoleFlags::default(),
    };

    let mut party = Party {
        partial: PartialParty {
            id: party_id,
            name: form.name,
            description: form.description,
        },
        avatar: None,
        banner: Nullable::Null,
        default_room: room_id,
        position: None,
        security: form.security,
        owner: auth.user_id,
        roles: Vec::new(),
        emotes: Vec::new(),
        pin_folders: Vec::new(),
    };

    let mut db = state.db.write.get().await?;
    let t = db.transaction().await?;

    // insert party first to avoid foreign key issues
    t.execute2(thorn::sql! {
        use schema::*;

        INSERT INTO Party (
            Id, Name, Description, OwnerId, DefaultRoom
        ) VALUES (
            #{&party.id           => Party::Id          },
            #{&party.name         => Party::Name        },
            #{&party.description  => Party::Description },
            #{&party.owner        => Party::OwnerId     },
            #{&party.default_room => Party::DefaultRoom }
        )
    }?)
    .await?;

    let position = {
        #[rustfmt::skip]
        let row = t.query_one2(thorn::sql! {
            use schema::*;

            SELECT MAX(PartyMembers.Position) AS @MaxPosition
            FROM PartyMembers WHERE PartyMembers.UserId = #{&auth.user_id => PartyMembers::UserId}
        }?).await?;

        match row.max_position::<Option<i16>>()? {
            Some(max_position) => max_position + 1,
            None => 0,
        }
    };

    party.position = Some(position);

    let [perm1, perm2] = default_role.permissions.to_i64();

    // NOTE: This is used to avoid lifetime issues
    futures::future::try_join3(
        t.execute2(thorn::sql! {
            use schema::*;

            INSERT INTO PartyMembers (
                PartyId, UserId, Position
            ) VALUES (
                #{&party.id     => PartyMembers::PartyId  },
                #{&auth.user_id => PartyMembers::UserId   },
                #{&position     => PartyMembers::Position }
            )
        }?),
        t.execute2(thorn::sql! {
            use schema::*;

            INSERT INTO Roles (
                Id, Name, PartyId, Permissions1, Permissions2
            ) VALUES (
                #{&default_role.id   => Roles::Id           },
                #{&default_role.name => Roles::Name         },
                #{&party.id          => Roles::PartyId      },
                #{&perm1             => Roles::Permissions1 },
                #{&perm2             => Roles::Permissions2 }
            )
        }?),
        t.execute2(thorn::sql! {
            use schema::*;

            INSERT INTO Rooms (
                Id, PartyId, Name, Position, Flags
            ) VALUES (
                #{&room_id              => Rooms::Id       },
                #{&party.id             => Rooms::PartyId  },
                #{&"general"            => Rooms::Name     },
                #{&0i16                 => Rooms::Position },
                #{&RoomFlags::DEFAULT   => Rooms::Flags    }
            )
        }?),
    )
    .await?;

    t.commit().await?;

    party.roles.push(default_role);

    Ok(party)
}
