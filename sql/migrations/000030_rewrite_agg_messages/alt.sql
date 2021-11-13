EXPLAIN ANALYZE VERBOSE WITH m AS (
   SELECT id
   FROM messages
   WHERE (messages.room_id = 302648171706812067) AND ((flags & 1) = 0) AND id < 959484642052538622
   ORDER BY id DESC LIMIT 100
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
    reactions.r
FROM

lantern.messages INNER JOIN m ON messages.id = m.id

INNER JOIN lantern.agg_users users ON users.id = messages.user_id
INNER JOIN lantern.rooms ON rooms.id = messages.room_id

LEFT JOIN lantern.agg_members member ON (member.user_id = messages.user_id AND member.party_id = rooms.party_id)

LEFT JOIN LATERAL (
    SELECT
        jsonb_agg(jsonb_build_object(
            'id', files.id,
            'size', files.size,
            'flags', files.flags,
            'name', files.name,
            'mime', files.mime
        )) AS meta,
        array_agg(files.preview) AS preview
    FROM
        lantern.attachments INNER JOIN lantern.files ON files.id = attachments.file_id
    WHERE attachments.message_id = m.id
) agg_attachments ON TRUE

LEFT JOIN LATERAL (
    SELECT
       array_agg(CASE WHEN mentions.user_id IS NOT NULL THEN 1
                      WHEN mentions.role_id IS NOT NULL THEN 2
                      WHEN mentions.room_id IS NOT NULL THEN 3
                 END) AS kinds,
       array_agg(COALESCE(mentions.user_id, mentions.role_id, mentions.room_id)) AS ids
    FROM mentions WHERE msg_id = m.id
) agg_mentions ON TRUE

LEFT JOIN LATERAL (
    SELECT jsonb_agg(jsonb_build_object(
        'emote', reactions.emote_id,
        'own', 302647985827741696 = ANY(reactions.user_ids),
        'count', array_length(reactions.user_ids, 1)
    )) AS r
    FROM lantern.reactions
    WHERE msg_id = m.id
) reactions ON TRUE