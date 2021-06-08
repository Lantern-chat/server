CREATE OR REPLACE VIEW lantern.agg_overwrites(room_id, user_id, role_id, user_allow, user_deny, allow, deny) AS

-- simple per-user overwrites
SELECT
    overwrites.room_id,
    overwrites.user_id,
    overwrites.role_id,
    overwrites.allow,
    overwrites.deny, 0, 0
FROM lantern.overwrites
WHERE user_id IS NOT NULL

UNION ALL

-- per-role overwrites where the user has that role, automatically filtered by using role_members.user_id
SELECT
    overwrites.room_id,
    role_members.user_id,
    overwrites.role_id,
    0, 0,
    overwrites.allow,
    overwrites.deny

FROM lantern.overwrites INNER JOIN lantern.role_members ON overwrites.role_id = role_members.role_id

UNION ALL

-- @everyone role overrides, which are roles with the same id as the party
SELECT
    overwrites.room_id,
    party_member.user_id,
    overwrites.role_id,
    0, 0, overwrites.allow, overwrites.deny

FROM
    lantern.party_member INNER JOIN
        lantern.roles INNER JOIN
            lantern.overwrites INNER JOIN lantern.rooms ON rooms.id = overwrites.room_id
        ON roles.id = rooms.party_id AND roles.id = overwrites.role_id
    ON party_member.party_id = rooms.party_id;


CREATE OR REPLACE VIEW lantern.agg_room_perms(room_id, user_id, perms) AS
SELECT
    rooms.id AS room_id,
    party_member.user_id,
--    roles.permissions, deny, allow, user_deny, user_allow
--    bit_or(roles.permissions), COALESCE(bit_or(deny), 0), COALESCE(bit_or(allow), 0), COALESCE(bit_or(user_deny), 0), COALESCE(bit_or(user_allow), 0)
    (((bit_or(permissions) & ~COALESCE(bit_or(deny), 0)) | COALESCE(bit_or(allow), 0)) & ~COALESCE(bit_or(user_deny), 0)) | COALESCE(bit_or(user_allow), 0) |
       bit_or(CASE WHEN party.owner_id = party_member.user_id THEN -1 ELSE 0 END) AS perms

FROM
    lantern.agg_overwrites RIGHT JOIN
        lantern.roles RIGHT JOIN
            lantern.rooms INNER JOIN
                lantern.party INNER JOIN lantern.party_member ON party_member.party_id = party.id
            ON rooms.party_id = party.id
        ON roles.id = party.id
    ON agg_overwrites.room_id = rooms.id AND agg_overwrites.user_id = party_member.user_id
GROUP BY party_member.user_id, rooms.id;