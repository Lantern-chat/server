CREATE TABLE lantern.messages (
    -- Snowflake ID, contains created_at timestamp
    id          bigint      NOT NULL,
    user_id     bigint      NOT NULL,
    room_id     bigint      NOT NULL,
    thread_id   bigint,
    updated_at  timestamp               DEFAULT now(),
    edited_at   timestamp,
    flags       smallint    NOT NULL    DEFAULT 0,
    content     text, -- NOTE: THIS MUST BE THE ONLY VARIABLE FIELD IN TABLE, due to TOAST stuff

    CONSTRAINT messages_pk PRIMARY KEY (id)
);
ALTER TABLE lantern.messages OWNER TO postgres;

ALTER TABLE lantern.messages SET (toast_tuple_target = 128);

-- Since id is a snowflake, it's always sorted by time
-- so index with btree for the times when we need to fetch by timestamps
CREATE UNIQUE INDEX msg_id_idx ON lantern.messages USING btree (id);

-- Index user and room ids for faster lookups
CREATE INDEX msg_user_idx ON lantern.messages USING hash (user_id);
CREATE INDEX msg_room_idx ON lantern.messages USING hash (room_id);

ALTER TABLE lantern.messages ADD CONSTRAINT room_fk FOREIGN KEY (room_id)
    REFERENCES lantern.rooms (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE; -- If room is deleted, delete all messages in room

ALTER TABLE lantern.messages ADD CONSTRAINT user_fk FOREIGN KEY (user_id)
    REFERENCES lantern.users (id) MATCH FULL
    ON DELETE SET NULL ON UPDATE CASCADE; -- If user is deleted, just set to NULL



-- Message attachments association map
CREATE TABLE lantern.attachments (
    message_id  bigint NOT NULL,
    file_id     bigint NOT NULL,

    CONSTRAINT attachment_pk PRIMARY KEY (message_id, file_id)
);
ALTER TABLE lantern.attachments OWNER TO postgres;

CREATE INDEX attachment_msg_idx ON lantern.attachments USING hash(message_id);
CREATE INDEX attachment_file_idx ON lantern.attachments USING hash(file_id);

ALTER TABLE lantern.attachments ADD CONSTRAINT message_fk FOREIGN KEY (message_id)
    REFERENCES lantern.messages (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE; -- Delete attachments on message deletion

-- Each attachment has a unique file
ALTER TABLE lantern.attachments ADD CONSTRAINT attachment_uq UNIQUE (file_id);
ALTER TABLE lantern.attachments ADD CONSTRAINT file_fk FOREIGN KEY (file_id)
    REFERENCES lantern.files (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE; -- On file deletion, delete attachment entry