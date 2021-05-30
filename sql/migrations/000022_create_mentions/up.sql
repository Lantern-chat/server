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
CREATE INDEX mention_user_idx ON lantern.mentions USING hash (user_id);

CREATE INDEX mention_role_idx ON lantern.mentions USING hash (role_id);

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