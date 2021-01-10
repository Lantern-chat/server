-- Backing file table for all attachments, avatars and so forth
CREATE TABLE lantern.files (
    -- Snowflake ID
    id      bigint      NOT NULL,

    -- filename given at upload
    name    text        NOT NULL,

    -- path used by the server
    path    text        NOT NULL,

    -- blurhash preview (first frame of video if video)
    -- this shouldn't be too large, less than 128 bytes
    preview bytea,

    -- MIME type
    mime    text,

    -- Size of file in bytes
    size    int         NOT NULL,

    -- SHA3-256 hash for error-checking and deduplication
    sha3    bytea       NOT NULL UNIQUE,

    CONSTRAINT file_pk PRIMARY KEY (id)
);

CREATE INDEX CONCURRENTLY file_hash_idx ON lantern.files USING HASH(sha3);