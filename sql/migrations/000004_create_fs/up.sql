-- Backing file table for all attachments, avatars and so forth
CREATE TABLE lantern.files (
    -- Snowflake ID
    id      bigint      NOT NULL,

    -- Encryption Nonce
    nonce   bigint,

    -- Size of file in bytes
    size    int         NOT NULL,

    -- Offset of file write
    "offset"  int       NOT NULL DEFAULT 0,

    -- Bitflags for state
    flags   smallint    NOT NULL,

    -- filename given at upload
    name    text        NOT NULL,

    -- MIME type
    mime    text,

    -- blurhash preview (first frame of video if video)
    -- this shouldn't be too large, less than 128 bytes
    preview bytea,

    CONSTRAINT file_pk PRIMARY KEY (id)
);

CREATE INDEX file_idx ON lantern.files USING hash(id);

CREATE OR REPLACE PROCEDURE lantern.upsert_file(
    _id bigint,
    _name text,
    _preview bytea,
    _mime text,
    _size int,
    _offset int,
    _flags smallint
)
LANGUAGE plpgsql AS
$$
BEGIN
    INSERT INTO lantern.files (id, name, preview, mime, size, "offset", flags)
    VALUES (_id, _name, _preview, _mime, _size, _offset, _flags)
    ON CONFLICT ON CONSTRAINT file_pk DO
        UPDATE SET preview  = _preview,
                   "offset" = _offset,
                   flags    = _flags;
END
$$;