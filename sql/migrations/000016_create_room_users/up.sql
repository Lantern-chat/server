CREATE TABLE lantern.room_users (
    room_id     bigint      NOT NULL,
    user_id     bigint      NOT NULL,

    -- applicable for slowmode
    last_msg    timestamp,
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