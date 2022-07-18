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
    preferences,
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
    users.preferences,
    agg_presence.flags,
    agg_presence.updated_at,
    agg_presence.activity

FROM
    lantern.users LEFT JOIN lantern.agg_presence ON agg_presence.user_id = users.id
;

CREATE OR REPLACE VIEW lantern.agg_members(
    user_id,
    party_id,
    nickname,
    flags,
    joined_at,
    role_ids
) AS
SELECT
    party_member.user_id,
    party_member.party_id,
    party_member.nickname,
    party_member.flags,
    party_member.joined_at,
    agg_roles.roles
FROM
    lantern.party_member
    LEFT JOIN LATERAL (
        SELECT
            ARRAY_AGG(role_id) as roles
        FROM
            lantern.role_members INNER JOIN lantern.roles
            ON  roles.id = role_members.role_id
            AND roles.party_id = party_member.party_id
            AND role_members.user_id = party_member.user_id
    ) agg_roles ON TRUE
;

CREATE OR REPLACE VIEW lantern.agg_user_associations(user_id, other_id) AS
SELECT user_id, friend_id FROM lantern.agg_friends
UNION ALL
SELECT my.user_id, o.user_id FROM
    lantern.party_member my INNER JOIN lantern.party_member o ON (o.party_id = my.party_id)
;