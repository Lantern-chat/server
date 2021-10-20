CREATE OR REPLACE VIEW lantern.agg_presence(
    user_id,
    flags,
    updated_at,
    activity
) AS
SELECT DISTINCT ON (user_id)
    user_id,
    flags,
    updated_at,
    activity
   FROM lantern.user_presence
  ORDER BY user_id, updated_at DESC
;

CREATE OR REPLACE VIEW lantern.agg_users(
    id,
    discriminator,
    email,
    flags,
    username,
    biography,
    custom_status,
    preferences,
    avatar_id,
    presence_flags,
    presence_updated_at,
    presence_activity
)
AS

SELECT
    users.id,
    users.discriminator,
    users.email,
    users.flags,
    users.username,
    users.biography,
    users.custom_status,
    users.preferences,
    user_avatars.file_id,
    agg_presence.flags, agg_presence.updated_at, agg_presence.activity

FROM
    lantern.user_avatars RIGHT JOIN
        lantern.users LEFT JOIN lantern.agg_presence ON agg_presence.user_id = users.id
    ON (user_avatars.user_id = users.id AND user_avatars.party_id IS NULL)
;

CREATE OR REPLACE VIEW lantern.agg_members(
    user_id,
    party_id,
    nickname,
    flags,
    avatar_id,
    joined_at,
    role_ids
) AS

SELECT
    party_member.user_id,
    party_member.party_id,
    party_member.nickname,
    party_member.flags,
    user_avatars.file_id,
    party_member.joined_at,
    ARRAY(
    SELECT
        role_id
    FROM
        lantern.role_members INNER JOIN lantern.roles
        ON roles.id = role_members.role_id AND
           roles.party_id = party_member.party_id
    WHERE
        role_members.user_id = party_member.user_id
    )

FROM
    lantern.party_member LEFT JOIN lantern.user_avatars ON
        (user_avatars.user_id = party_member.user_id AND user_avatars.party_id = party_member.party_id)
;