CREATE TABLE lantern.room_users (
    room_id     bigint      NOT NULL,
    user_id     bigint      NOT NULL,

    -- applicable for notifications
    last_read   bigint,
    -- applicable for slowmode
    last_sent   bigint,
    -- muted users cannot speak
    muted       boolean,

    CONSTRAINT room_users_pk PRIMARY KEY (room_id, user_id)
);
ALTER TABLE lantern.room_users OWNER TO postgres;

ALTER TABLE lantern.room_users ADD CONSTRAINT room_id_fk FOREIGN KEY (room_id)
    REFERENCES lantern.rooms (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;

ALTER TABLE lantern.room_users ADD CONSTRAINT user_id_fk FOREIGN KEY (user_id)
    REFERENCES lantern.users (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;

-- On delete, don't update this as the stored id still contains the timestamp
ALTER TABLE lantern.room_users ADD CONSTRAINT last_read_fk FOREIGN KEY (last_read)
    REFERENCES lantern.messages (id) MATCH FULL
    ON DELETE NO ACTION ON UPDATE CASCADE;

-- On delete, don't update this as the stored id still contains the timestamp
ALTER TABLE lantern.room_users ADD CONSTRAINT last_sent_fk FOREIGN KEY (last_sent)
    REFERENCES lantern.messages (id) MATCH FULL
    ON DELETE NO ACTION ON UPDATE CASCADE;