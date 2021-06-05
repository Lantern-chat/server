CREATE OR REPLACE VIEW agg_roomperms(room_id, user_id, user_allow, user_deny, allow, deny) AS
SELECT
    overwrites.room_id,
    overwrites.user_id,
    (CASE WHEN overwrites.user_id IS NOT NULL THEN allow ELSE 0 END) as user_allow,
    (CASE WHEN overwrites.user_id IS NOT NULL THEN deny ELSE 0 END) as user_deny,
    (CASE WHEN overwrites.user_id IS NULL THEN allow ELSE 0 END) as allow,
    (CASE WHEN overwrites.user_id IS NULL THEN deny ELSE 0 END) as deny
FROM
    lantern.overwrites LEFT JOIN lantern.role_members
    ON role_members.role_id = overwrites.role_id;

CREATE OR REPLACE VIEW agg_partyperms_from_room(party_id, owner_id, room_id, user_id, permissions) AS
SELECT
    party.id,
    owner_id,
    rooms.id,
    party_member.user_id,
    roles.permissions
FROM
lantern.party_member LEFT JOIN
    lantern.role_members RIGHT JOIN
        lantern.roles INNER JOIN
            lantern.rooms INNER JOIN lantern.party
            ON rooms.party_id = party.id
        ON roles.party_id = party.id
    ON role_members.role_id = roles.id
ON (party_member.user_id = role_members.user_id) IS NOT FALSE;

CREATE OR REPLACE FUNCTION get_room_permissions(
    user_id bigint,
    room_id bigint
)
RETURNS TABLE(perm bigint)
LANGUAGE SQL IMMUTABLE ROWS 1
AS $$
WITH "with_agg_room_perms" AS (
    SELECT
        "agg_roomperms"."user_allow" AS "user_allow",
        "agg_roomperms"."user_deny" AS "user_deny",
        "agg_roomperms"."allow" AS "allow",
        "agg_roomperms"."deny" AS "deny"
    FROM "lantern"."agg_roomperms"
    WHERE
        (("agg_roomperms"."room_id" = $2) AND
        (("agg_roomperms"."user_id" = $1) IS NOT FALSE))
),
"with_agg_party_perms" AS (
    SELECT
        "agg_partyperms_from_room"."owner_id" AS "owner_id",
        BIT_OR("agg_partyperms_from_room"."permissions") AS "base"
    FROM
        "lantern"."agg_partyperms_from_room"
    WHERE (("agg_partyperms_from_room"."room_id" = $2) AND
           ("agg_partyperms_from_room"."user_id" = $1))
    GROUP BY "agg_partyperms_from_room"."owner_id"
) SELECT
    (CASE WHEN ("with_agg_party_perms"."owner_id" = $1 ) THEN -1
    ELSE ((((BIT_OR("with_agg_party_perms"."base")
         & (~BIT_OR("with_agg_room_perms"."deny")))
         |   BIT_OR("with_agg_room_perms"."allow"))
         & (~BIT_OR("with_agg_room_perms"."user_deny")))
         |   BIT_OR("with_agg_room_perms"."user_allow")) END)
FROM "with_agg_room_perms", "with_agg_party_perms"
GROUP BY "with_agg_party_perms"."owner_id"
$$;