CREATE TABLE lantern.mentions (
    msg_id      bigint NOT NULL,

    user_id     bigint,
    role_id     bigint,
    room_id     bigint
);
ALTER TABLE lantern.mentions OWNER TO postgres;

-- allow to find and sort by msg id
CREATE INDEX mention_msg_idx ON lantern.mentions USING btree (msg_id);

-- allow a user to search for their own mentions
CREATE INDEX mention_user_idx ON lantern.mentions USING btree (user_id) WHERE user_id IS NOT NULL;
CREATE INDEX mention_role_idx ON lantern.mentions USING btree (role_id) WHERE role_id IS NOT NULL;

ALTER TABLE lantern.mentions ADD CONSTRAINT msg_fk FOREIGN KEY (msg_id)
    REFERENCES lantern.messages (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;

ALTER TABLE lantern.mentions ADD CONSTRAINT user_fk FOREIGN KEY (user_id)
    REFERENCES lantern.users (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;

ALTER TABLE lantern.mentions ADD CONSTRAINT role_fk FOREIGN KEY (role_id)
    REFERENCES lantern.roles (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;

ALTER TABLE lantern.mentions ADD CONSTRAINT room_fk FOREIGN KEY (room_id)
    REFERENCES lantern.rooms (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;

ALTER TABLE lantern.mentions ADD CONSTRAINT check_all CHECK (
    1 = (user_id IS NOT NULL)::int4 + (role_id IS NOT NULL)::int4 + (room_id IS NOT NULL)::int4
);

CREATE OR REPLACE VIEW lantern.agg_mentions AS
SELECT mentions.msg_id,
       array_agg(CASE WHEN mentions.user_id IS NOT NULL THEN 1
                      WHEN mentions.role_id IS NOT NULL THEN 2
                      WHEN mentions.room_id IS NOT NULL THEN 3
                 END) AS kinds,
       array_agg(COALESCE(mentions.user_id, mentions.role_id, mentions.room_id)) AS ids
FROM mentions GROUP BY msg_id;