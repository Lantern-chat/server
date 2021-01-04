-- Tracks each users last read message in any room, across devices
CREATE TABLE lantern.lrm (
    message_id  bigint NOT NULL,
    user_id     bigint NOT NULL,
    room_id     bigint NOT NULL,

    CONSTRAINT last_read_message_pk PRIMARY KEY (message_id, user_id, room_id)
);
ALTER TABLE lantern.lrm OWNER TO postgres;

-- On delete, don't update this as the stored id still contains the timestamp
ALTER TABLE lantern.lrm ADD CONSTRAINT message_fk FOREIGN KEY (message_id)
    REFERENCES lantern.messages (id) MATCH FULL
    ON DELETE NO ACTION ON UPDATE CASCADE;

ALTER TABLE lantern.lrm ADD CONSTRAINT lrm_uq UNIQUE (message_id);

ALTER TABLE lantern.lrm ADD CONSTRAINT user_fk FOREIGN KEY (user_id)
    REFERENCES lantern.users (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;

ALTER TABLE lantern.lrm ADD CONSTRAINT room_fk FOREIGN KEY (room_id)
    REFERENCES lantern.rooms (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;
