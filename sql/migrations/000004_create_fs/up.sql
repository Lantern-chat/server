-- Backing file table for all attachments, avatars and so forth
CREATE TABLE lantern.files (
    -- Snowflake ID
    id      bigint      NOT NULL,

    -- filename given at upload
    name    text        NOT NULL,

    -- blurhash preview (first frame of video if video)
    -- this shouldn't be too large, less than 128 bytes
    preview bytea,

    -- MIME type
    mime    text,

    -- Size of file in bytes
    size    int         NOT NULL,

    -- Offset of file write
    "offset"  int         NOT NULL DEFAULT 0,

    -- Bitflags for state
    flags   smallint    NOT NULL,

    -- SHA3-256 hash for error-checking and deduplication
    sha3    bytea       NOT NULL UNIQUE,

    CONSTRAINT file_pk PRIMARY KEY (id)
);

CREATE INDEX CONCURRENTLY file_hash_idx ON lantern.files USING HASH(sha3);

CREATE OR REPLACE PROCEDURE lantern.upsert_file(
    _id bigint,
    _name text,
    _preview bytea,
    _mime text,
    _size int,
    _offset int,
    _flags smallint,
    _sha3 bytea
)
LANGUAGE plpgsql AS
$$
BEGIN
    INSERT INTO lantern.files (id, name, preview, mime, size, "offset", flags, sha3)
    VALUES (_id, _name, _preview, _mime, _size, _offset, _flags, _sha3)
    ON CONFLICT ON CONSTRAINT file_pk DO
        UPDATE SET name = _name, preview = _preview, mime = _mime,
                   size = _size, "offset" = _offset, flags = _flags,
                   sha3 = _sha3;
END
$$;