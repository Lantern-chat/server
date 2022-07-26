CREATE TABLE lantern.pin_tags (
    id bigint NOT NULL,

    icon_id bigint, -- emote id for icon

    -- might include color
    flags int NOT NULL DEFAULT 0,

    name text NOT NULL,
    description text,

    CONSTRAINT pin_tags_pk PRIMARY KEY (id)
);
ALTER TABLE lantern.pin_tags OWNER TO postgres;

ALTER TABLE lantern.pin_tags ADD CONSTRAINT icon_fk FOREIGN KEY (icon_id)
    REFERENCES lantern.emotes (id) MATCH FULL
    ON DELETE SET NULL ON UPDATE CASCADE;

-- create a pin_tags array on messages and use a GIN index for fast searching by pin_tag
ALTER TABLE lantern.messages ADD COLUMN pin_tags bigint[];
CREATE INDEX message_pin_tag_idx ON lantern.messages USING GIN (pin_tags) WHERE pin_tags IS NOT NULL;

-- modify agg_messages to include pin_tags
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
    pin_ids,
    content,
    role_tags,
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
    messages.pin_tags,
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