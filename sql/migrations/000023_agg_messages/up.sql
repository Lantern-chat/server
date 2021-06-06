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
    messages.content,
    (SELECT array_agg(role_members.role_id)
        FROM role_members JOIN roles ON role_members.role_id = roles.id
        WHERE role_members.user_id = messages.user_id AND roles.party_id = party_member.party_id
    ) AS roles
FROM
lantern.agg_mentions RIGHT JOIN
    lantern.party_member RIGHT JOIN
        lantern.rooms INNER JOIN
            lantern.users INNER JOIN lantern.messages ON users.id = messages.user_id
        ON rooms.id = messages.room_id
    ON (party_member.user_id = messages.user_id AND party_member.party_id = rooms.party_id)
ON agg_mentions.msg_id = messages.id;