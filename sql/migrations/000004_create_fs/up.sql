-- Backing file table for all attachments, avatars and so forth
CREATE TABLE lantern.files (
    -- Snowflake ID
    id      bigint      NOT NULL,

    user_id bigint      NOT NULL,

    -- Encryption Nonce
    nonce   bigint,

    -- Size of file in bytes
    size    int         NOT NULL,

    width   int,
    height  int,

    -- Bitflags for state
    flags   smallint    NOT NULL,

    -- filename given at upload
    name    text        NOT NULL,

    -- MIME type
    mime    text,

    -- SHA-1 hash of completed file
    sha1    bytea,

    -- blurhash preview (first frame of video if video)
    -- this shouldn't be too large, less than 128 bytes
    preview bytea,

    CONSTRAINT file_pk PRIMARY KEY (id)
);

CREATE INDEX file_idx ON lantern.files USING btree(user_id, id) INCLUDE (size);

ALTER TABLE lantern.files ADD CONSTRAINT user_fk FOREIGN KEY (user_id)
    REFERENCES lantern.users (id) MATCH FULL
    ON DELETE RESTRICT ON UPDATE CASCADE;