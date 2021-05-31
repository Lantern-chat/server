CREATE OR REPLACE VIEW lantern.agg_messages AS
SELECT
    messages.id AS msg_id,
    messages.user_id,
    rooms.party_id,
    messages.room_id,
    party_member.nickname,
    users.username,
    users.discriminator,
    agg_mentions.kinds AS mention_kinds,
    agg_mentions.ids AS mention_ids,
    messages.edited_at,
    messages.flags,
    messages.content
FROM
lantern.agg_mentions RIGHT JOIN
    lantern.party_member RIGHT JOIN
        lantern.rooms INNER JOIN
            lantern.users INNER JOIN lantern.messages ON users.id = messages.user_id
        ON rooms.id = messages.room_id
    ON (party_member.user_id = messages.user_id AND party_member.party_id = rooms.party_id)
ON agg_mentions.msg_id = messages.id;