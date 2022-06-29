DROP VIEW IF EXISTS lantern.agg_messages;

CREATE OR REPLACE VIEW lantern.agg_messages(
    msg_id,
    user_id,
    room_id,
    party_id,
    kind,
    nickname,
    username,
    discriminator,
    user_flags,
    avatar_id,
    profile_bits,
    thread_id,
    edited_at,
    message_flags,
    mention_kinds,
    mention_ids,
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
    messages.kind,
    member.nickname,
    users.username,
    users.discriminator,
    users.flags,
    profile.avatar_id,
    profile.bits, -- Might be NULL if the user has no profile at all
    messages.thread_id,
    messages.edited_at,
    messages.flags,
    agg_mentions.kinds,
    agg_mentions.ids,
    messages.content,
    member.role_ids,
    agg_attachments.meta,
    agg_attachments.preview
FROM

lantern.messages

INNER JOIN lantern.agg_users users ON users.id = messages.user_id
INNER JOIN lantern.rooms ON rooms.id = messages.room_id

LEFT JOIN lantern.agg_profiles profile ON (
    profile.user_id = messages.user_id AND
    -- NOTE: profile.party_id is allowed to be NULL
    (profile.party_id = rooms.party_id IS NOT FALSE)
)

LEFT JOIN lantern.agg_members member ON (member.user_id = messages.user_id AND member.party_id = rooms.party_id)
LEFT JOIN lantern.agg_attachments ON agg_attachments.msg_id = messages.id
LEFT JOIN lantern.agg_mentions ON agg_mentions.msg_id = messages.id
;