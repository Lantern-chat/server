DROP VIEW IF EXISTS lantern.agg_messages;

CREATE OR REPLACE VIEW lantern.agg_messages(
    msg_id,
    user_id,
    room_id,
    party_id,
    nickname,
    username,
    discriminator,
    user_flags,
    avatar_id,
    mention_kinds,
    mention_ids,
    edited_at,
    message_flags,
    content,
    role_ids,
    attachment_meta,
    attachment_preview,
) AS
SELECT
    messages.id,
    messages.user_id,
    messages.room_id,
    rooms.party_id,
    member.nickname,
    users.username,
    users.discriminator,
    users.flags,
    COALESCE(users.avatar_id, member.avatar_id),
    agg_mentions.kinds AS mention_kinds,
    agg_mentions.ids AS mention_ids,
    messages.edited_at,
    messages.flags as message_flags,
    messages.content,
    member.role_ids,
    agg_attachments.meta,
    agg_attachments.preview
FROM

lantern.messages

INNER JOIN lantern.agg_users users ON users.id = messages.user_id
INNER JOIN lantern.rooms ON rooms.id = messages.room_id

LEFT JOIN lantern.agg_members member ON (member.user_id = messages.user_id AND member.party_id = rooms.party_id)
LEFT JOIN lantern.agg_attachments ON agg_attachments.msg_id = messages.id
LEFT JOIN lantern.agg_mentions ON agg_mentions.msg_id = messages.id
;

DROP FUNCTION IF EXISTS lantern.select_messages(bigint, bigint, bigint, boolean);
DROP FUNCTION IF EXISTS lantern.select_messages(bigint, bigint, bigint, integer, boolean);

CREATE OR REPLACE FUNCTION lantern.select_messages(
    _room_id bigint,
    _user_id bigint,
    _last_id bigint,
    _limit integer,
    _ascending boolean
)
RETURNS TABLE (
    msg_id bigint,
    user_id bigint,
    room_id bigint,
    party_id bigint,
    nickname varchar(256),
    username varchar(64),
    discriminator uint2,
    user_flags integer,
    avatar_id bigint,
    mention_kinds integer[],
    mention_ids bigint[],
    edited_at timestamp,
    message_flags smallint,
    content text,
    role_ids bigint[],
    attachment_meta jsonb,
    attachment_preview bytea[],
    reactions jsonb
) AS
$$
BEGIN

RETURN QUERY
WITH m AS (
    SELECT id FROM messages WHERE (messages.room_id = _room_id) AND ((flags & 1) = 0) AND
   ((id < _last_id AND _ascending) OR (id > _last_id AND NOT _ascending))

   ORDER BY
   CASE WHEN _ascending THEN id END DESC,
   CASE WHEN NOT _ascending THEN id END ASC

   LIMIT _limit
)
SELECT
    "agg_messages"."msg_id",
    "agg_messages"."user_id",
    "agg_messages"."room_id",
    "agg_messages"."party_id",
    "agg_messages"."nickname",
    "agg_messages"."username",
    "agg_messages"."discriminator",
    "agg_messages"."user_flags",
    "agg_messages"."avatar_id",
    "agg_messages"."mention_kinds",
    "agg_messages"."mention_ids",
    "agg_messages"."edited_at",
    "agg_messages"."message_flags",
    "agg_messages"."content",
    "agg_messages"."role_ids",
    "agg_messages"."attachment_meta",
    "agg_messages"."attachment_preview",
     to_jsonb(reactions.r)
FROM "lantern"."agg_messages" INNER JOIN m ON agg_messages.msg_id = m.id
LEFT JOIN LATERAL (
    SELECT reactions.msg_id, array_agg(jsonb_build_object(
        'emote', reactions.emote_id,
        'own', _user_id = ANY(reactions.user_ids),
        'count', array_length(reactions.user_ids, 1)
    )) AS r
    FROM lantern.reactions
    GROUP BY reactions.msg_id
) reactions ON reactions.msg_id = m.id;

END;
$$ LANGUAGE plpgsql;



DROP FUNCTION IF EXISTS lantern.select_messages_alt(bigint, bigint, bigint, boolean);
DROP FUNCTION IF EXISTS lantern.select_messages_alt(bigint, bigint, bigint, integer, boolean);

CREATE OR REPLACE FUNCTION lantern.select_messages_alt(
    _room_id bigint,
    _user_id bigint,
    _last_id bigint,
    _limit integer,
    _ascending boolean
)
RETURNS TABLE (
    msg_id bigint,
    user_id bigint,
    room_id bigint,
    party_id bigint,
    nickname varchar(256),
    username varchar(64),
    discriminator uint2,
    user_flags integer,
    avatar_id bigint,
    mention_kinds integer[],
    mention_ids bigint[],
    edited_at timestamp,
    message_flags smallint,
    content text,
    role_ids bigint[],
    attachment_meta jsonb,
    attachment_preview bytea[],
    reactions jsonb
) AS
$$
BEGIN

RETURN QUERY
WITH m AS (
    SELECT messages.id
    FROM messages WHERE (messages.room_id = _room_id) AND ((messages.flags & 1) = 0) AND
   ((messages.id < _last_id AND _ascending) OR (messages.id > _last_id AND NOT _ascending))

   ORDER BY
   CASE WHEN _ascending THEN messages.id END DESC,
   CASE WHEN NOT _ascending THEN messages.id END ASC

   LIMIT _limit
)
SELECT
    messages.id,
    messages.user_id,
    messages.room_id,
    rooms.party_id,
    member.nickname,
    users.username,
    users.discriminator,
    users.flags,
    COALESCE(users.avatar_id, member.avatar_id),
    agg_mentions.kinds AS mention_kinds,
    agg_mentions.ids AS mention_ids,
    messages.edited_at,
    messages.flags as message_flags,
    messages.content,
    member.role_ids,
    agg_attachments.meta,
    agg_attachments.preview,
    to_jsonb(reactions.r)
FROM

lantern.messages INNER JOIN m ON messages.id = m.id

INNER JOIN lantern.agg_users users ON users.id = messages.user_id
INNER JOIN lantern.rooms ON rooms.id = messages.room_id

LEFT JOIN lantern.agg_members member ON (member.user_id = messages.user_id AND member.party_id = rooms.party_id)
LEFT JOIN lantern.agg_attachments ON agg_attachments.msg_id = messages.id
LEFT JOIN lantern.agg_mentions ON agg_mentions.msg_id = messages.id

LEFT JOIN LATERAL (
    SELECT reactions.msg_id, array_agg(jsonb_build_object(
        'emote', reactions.emote_id,
        'own', _user_id = ANY(reactions.user_ids),
        'count', array_length(reactions.user_ids, 1)
    )) AS r
    FROM lantern.reactions
    GROUP BY reactions.msg_id
) reactions ON reactions.msg_id = messages.id;

END;
$$ LANGUAGE plpgsql;