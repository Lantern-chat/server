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
ALTER TABLE lantern.files OWNER TO postgres;

CREATE INDEX file_idx ON lantern.files USING btree(user_id, id) INCLUDE (size);

ALTER TABLE lantern.files ADD CONSTRAINT user_fk FOREIGN KEY (user_id)
    REFERENCES lantern.users (id) MATCH FULL
    ON DELETE RESTRICT ON UPDATE CASCADE;

CREATE TABLE lantern.user_assets (
    id          bigint      NOT NULL,

    -- original asset before processing
    file_id     bigint      NOT NULL,

    -- have one single blurhash preview for all versions of this asset
    preview     bytea,

    CONSTRAINT user_asset_pk PRIMARY KEY (id)
);
ALTER TABLE lantern.user_assets OWNER TO postgres;

CREATE INDEX user_asset_origina_file_idx ON lantern.user_assets (file_id);

CREATE TABLE lantern.user_asset_files (
    asset_id    bigint      NOT NULL,
    file_id     bigint      NOT NULL,

    -- will contain info about file type and quality settings
    flags       smallint    NOT NULL,

    CONSTRAINT user_asset_files_pk PRIMARY KEY (asset_id, file_id)
);
ALTER TABLE lantern.user_asset_files OWNER TO postgres;

-- TODO: Is this even necessary with such a simple table? The index itself has the same information as the actual table
CREATE INDEX user_asset_file_idx ON lantern.user_asset_files USING btree(asset_id, file_id) INCLUDE (flags);

--
-- Update existing tables with avatars fks
--

-- Add avatar to party
ALTER TABLE lantern.party ADD CONSTRAINT avatar_fk FOREIGN KEY (avatar_id)
    REFERENCES lantern.user_assets (id) MATCH FULL
    ON DELETE SET NULL ON UPDATE CASCADE; -- IMPORTANT: Only set NULL when deleting avatars, DO NOT CASCADE

-- Add avatar to rooms
ALTER TABLE lantern.rooms ADD CONSTRAINT avatar_fk FOREIGN KEY (avatar_id)
    REFERENCES lantern.user_assets (id) MATCH FULL
    ON DELETE SET NULL ON UPDATE CASCADE; -- IMPORTANT: Only set NULL when deleting avatars, DO NOT CASCADE


CREATE OR REPLACE VIEW lantern.agg_assets(
    asset_id,
    asset_flags,
    file_id,
    user_id,
    nonce,
    size,
    width,
    height,
    file_flags,
    file_name,
    mime,
    sha1,
    preview
) AS
SELECT
    assets.id,
    asset_files.flags,
    files.id,
    files.user_id,
    files.nonce,
    files.size,
    files.width,
    files.height,
    files.flags,
    files.name,
    files.mime,
    files.sha1,
    assets.preview
FROM
    lantern.user_asset_files asset_files
    INNER JOIN lantern.files ON (files.id = asset_files.file_id)
    RIGHT JOIN lantern.user_assets assets ON (asset_files.asset_id = assets.id)
;