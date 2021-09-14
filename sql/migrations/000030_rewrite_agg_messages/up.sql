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
    attachment_preview
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
lantern.agg_attachments RIGHT JOIN
    lantern.agg_mentions RIGHT JOIN
        lantern.agg_members member RIGHT JOIN
            lantern.rooms INNER JOIN
                lantern.messages INNER JOIN lantern.agg_users users ON users.id = messages.user_id
            ON rooms.id = messages.room_id
        ON (member.user_id = messages.user_id AND member.party_id = rooms.party_id)
    ON agg_mentions.msg_id = messages.id
ON agg_attachments.msg_id = messages.id
;
