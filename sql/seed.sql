----------------------------------------
-------------- SCHEMA ------------------
----------------------------------------

SET check_function_bodies = true;

CREATE SCHEMA lantern;
ALTER SCHEMA lantern OWNER TO postgres;

SET search_path TO pg_catalog, public, lantern;

ALTER SYSTEM SET enable_seqscan = 1;
ALTER SYSTEM SET jit = 0; -- honestly buggy, and we never create insane queries that need it anyway
ALTER SYSTEM SET random_page_cost = 1; -- Database on SSD
SELECT pg_reload_conf();

-- host table tracks migrations
CREATE TABLE lantern.host (
    migration int           NOT NULL,
    migrated  timestamptz   NOT NULL,

    CONSTRAINT migration_primary_key PRIMARY KEY (migration)
);

CREATE OR REPLACE FUNCTION lantern.array_diff(lhs anyarray, rhs anyarray)
    RETURNS anyarray
    LANGUAGE sql immutable
AS $$
    SELECT COALESCE(array_agg(elem), '{}')
    FROM UNNEST(lhs) elem
    WHERE elem <> ALL(rhs)
$$;

CREATE OR REPLACE FUNCTION lantern.array_uniq(arr anyarray)
    RETURNS anyarray
    LANGUAGE sql immutable
AS $$
    SELECT ARRAY( SELECT DISTINCT UNNEST(arr) )
$$;

CREATE DOMAIN lantern.uint2 AS int4
   CHECK(VALUE >= 0 AND VALUE < 65536);

-- THIS MUST MATCH `LanguageCode` in schema crate
CREATE OR REPLACE FUNCTION lantern.to_language(int2)
RETURNS regconfig
AS
$$
    SELECT CASE WHEN $1 = 0 THEN 'english'::regconfig
                WHEN $1 = 1 THEN 'arabic'::regconfig
                WHEN $1 = 2 THEN 'armenian'::regconfig
                WHEN $1 = 3 THEN 'basque'::regconfig
                WHEN $1 = 4 THEN 'catalan'::regconfig
                WHEN $1 = 5 THEN 'danish'::regconfig
                WHEN $1 = 6 THEN 'dutch'::regconfig
                WHEN $1 = 7 THEN 'finnish'::regconfig
                WHEN $1 = 8 THEN 'french'::regconfig
                WHEN $1 = 9 THEN 'german'::regconfig
                WHEN $1 = 10 THEN 'greek'::regconfig
                WHEN $1 = 11 THEN 'hindi'::regconfig
                WHEN $1 = 12 THEN 'hungarian'::regconfig
                WHEN $1 = 13 THEN 'indonesian'::regconfig
                WHEN $1 = 14 THEN 'irish'::regconfig
                WHEN $1 = 15 THEN 'italian'::regconfig
                WHEN $1 = 16 THEN 'lithuanian'::regconfig
                WHEN $1 = 17 THEN 'nepali'::regconfig
                WHEN $1 = 18 THEN 'norwegian'::regconfig
                WHEN $1 = 19 THEN 'portuguese'::regconfig
                WHEN $1 = 20 THEN 'romanian'::regconfig
                WHEN $1 = 21 THEN 'russian'::regconfig
                WHEN $1 = 22 THEN 'serbian'::regconfig
                WHEN $1 = 23 THEN 'simple'::regconfig
                WHEN $1 = 24 THEN 'spanish'::regconfig
                WHEN $1 = 25 THEN 'swedish'::regconfig
                WHEN $1 = 26 THEN 'tamil'::regconfig
                WHEN $1 = 27 THEN 'turkish'::regconfig
                WHEN $1 = 28 THEN 'yiddish'::regconfig
            ELSE 'english'::regconfig
        END
$$ LANGUAGE SQL IMMUTABLE;
COMMENT ON FUNCTION lantern.to_language IS 'Converts a language code into the equivalent regconfig language';

CREATE TYPE lantern.event_code AS ENUM (
    'message_create',
    'message_update',
    'message_delete',
    'typing_started',
    'user_updated',
    'self_updated',
    'presence_updated',
    'party_create',
    'party_update',
    'party_delete',
    'room_created',
    'room_updated',
    'room_deleted',
    'member_updated',
    'member_joined',
    'member_left',
    'member_ban',
    'member_unban',
    'role_created',
    'role_updated',
    'role_deleted',
    'invite_create',
    'message_react',
    'message_unreact',
    'profile_updated'
);

CREATE SEQUENCE lantern.event_id;

CREATE OR REPLACE FUNCTION lantern.combine_profile_bits(
    base_bits int,
    party_bits int,
    party_avatar bigint
) RETURNS int
AS
$$
    SELECT CASE
        WHEN party_bits IS NULL
            THEN base_bits
        ELSE
        -- Select lower 7 avatar bits
        (x'7F'::int & CASE
            WHEN party_avatar IS NOT NULL
                THEN party_bits
            ELSE base_bits
        END) |
        -- Select higher 25 banner bits
        (x'FFFFFF80'::int & CASE
            -- pick out 8th bit, which signifies whether to override banner color
            WHEN party_bits & 128 != 0
                THEN party_bits
            ELSE base_bits
        END)
    END
$$ LANGUAGE SQL IMMUTABLE;

----------------------------------------
-------------- TABLES ------------------
----------------------------------------

CREATE TABLE lantern.event_log (
    counter     bigint      NOT NULL DEFAULT nextval('lantern.event_id'),

    -- the snowflake ID of whatever this event is pointing to
    id          bigint      NOT NULL CONSTRAINT id_check CHECK (id > 0),

    -- If it's a party event, place the ID here for better throughput on application layer
    party_id    bigint,
    -- May be NULL even when the event
    room_id     bigint,

    code        lantern.event_code  NOT NULL
);
COMMENT ON COLUMN lantern.event_log.id IS 'The snowflake ID of whatever this event is pointing to';
COMMENT ON COLUMN lantern.event_log.counter IS 'Incrementing counter for sorting';

ALTER SEQUENCE lantern.event_id OWNED BY lantern.event_log.counter;

-- Notification rate-limiting table
CREATE TABLE lantern.event_log_last_notification (
    last_notif      timestamptz NOT NULL DEFAULT now(),
    max_interval    interval    NOT NULL DEFAULT INTERVAL '100 milliseconds'
);
COMMENT ON TABLE lantern.event_log_last_notification IS 'Notification rate-limiting table';

CREATE TABLE lantern.rate_limits (
    violations  integer     NOT NULL DEFAULT 0,
    addr        inet        NOT NULL
);

CREATE TABLE lantern.ip_bans (
    expires     timestamptz,
    address     inet,
    network     cidr
);

CREATE TABLE IF NOT EXISTS lantern.metrics (
    ts      timestamptz   NOT NULL DEFAULT now(),

    -- allocated memory usage, in bytes
    mem     bigint      NOT NULL,
    -- bytes uploaded by users since last metric
    upload  bigint      NOT NULL,

    -- requests since last metric
    reqs    int         NOT NULL,
    -- errors since last metric
    errs    int         NOT NULL,
    -- number of connected gateway users
    conns   int         NOT NULL,
    -- number of gateway events since last metric
    events  int         NOT NULL,

    -- latency percentiles
    p50     smallint    NOT NULL,
    p95     smallint    NOT NULL,
    p99     smallint    NOT NULL,

    CONSTRAINT metrics_pk PRIMARY KEY (ts)
);
COMMENT ON COLUMN lantern.metrics.mem IS 'allocated memory usage, in bytes';
COMMENT ON COLUMN lantern.metrics.upload IS 'bytes uploaded by users since last metric';
COMMENT ON COLUMN lantern.metrics.reqs IS 'requests since last metric';
COMMENT ON COLUMN lantern.metrics.errs IS 'errors since last metric';
COMMENT ON COLUMN lantern.metrics.conns IS 'number of connected gateway users';
COMMENT ON COLUMN lantern.metrics.events IS 'number of gateway events since last metric';
COMMENT ON COLUMN lantern.metrics.p50 IS '50th latency percently';
COMMENT ON COLUMN lantern.metrics.p95 IS '95th latency percentile';
COMMENT ON COLUMN lantern.metrics.p99 IS '99th latency percentile';

CREATE TABLE lantern.users (
    --- Snowflake id
    id              bigint              NOT NULL,
    deleted_at      timestamptz,
    last_active     timestamptz,
    dob             date                NOT NULL,
    flags           int                 NOT NULL    DEFAULT 0,
    -- 2-byte integer that can be displayed as 4 hex digits,
    -- actually stored as a 4-byte signed integer because Postgres doesn't support unsigned...
    discriminator   lantern.uint2       NOT NULL,
    username        text                NOT NULL,
    email           text                NOT NULL,
    passhash        text                NOT NULL,

    -- 2FA Secret key
    mfa_secret      bytea,
    mfa_backup      bytea,

    -- this is for client-side user preferences, which can be stored as JSON easily enough
    preferences     jsonb,

    CONSTRAINT users_pk PRIMARY KEY (id)
);
COMMENT ON COLUMN lantern.users.preferences IS 'this is for client-side user preferences, which can be stored as JSON easily enough';
COMMENT ON COLUMN lantern.users.discriminator IS '2-byte integer that can be displayed as 4 hex digits, actually stored as a 4-byte signed integer because Postgres doesn''t support unsigned...';

CREATE TABLE lantern.user_freelist (
    username        text            NOT NULL,
    discriminator   lantern.uint2   NOT NULL
);

-- User verification/reset tokens
CREATE TABLE lantern.user_tokens (
    id          bigint      NOT NULL,
    user_id     bigint      NOT NULL,
    expires     timestamptz   NOT NULL,
    kind        smallint    NOT NULL,
    token       bytea       NOT NULL,

    CONSTRAINT user_tokens_pk PRIMARY KEY (id)
);

CREATE TABLE lantern.party (
    id              bigint      NOT NULL,
    owner_id        bigint      NOT NULL,
    default_room    bigint      NOT NULL,
    -- packed party flags
    flags           bigint      NOT NULL DEFAULT 0,
    avatar_id       bigint,
    banner_id       bigint,
    deleted_at      timestamptz,
    name            text        NOT NULL,
    description     text,

    CONSTRAINT party_pk PRIMARY KEY (id)
);

-- Association map between parties and users
CREATE TABLE lantern.party_member (
    party_id    bigint          NOT NULL,
    user_id     bigint          NOT NULL,
    perms       bigint          NOT NULL DEFAULT 0,

    invite_id   bigint,
    joined_at   timestamptz     NOT NULL    DEFAULT now(),
    mute_until  timestamptz,
    flags       smallint        NOT NULL    DEFAULT 0,
    position    smallint        NOT NULL    DEFAULT 0,

    -- Composite primary key
    CONSTRAINT party_member_pk PRIMARY KEY (party_id, user_id)
);
COMMENT ON TABLE lantern.party_member IS 'Association map between parties and users';

CREATE TABLE lantern.rooms (
    id              bigint      NOT NULL,
    party_id        bigint,
    avatar_id       bigint,
    parent_id       bigint,
    deleted_at      timestamptz,
    position        smallint    NOT NULL,
    flags           smallint    NOT NULL    DEFAULT 0,
    name            text        NOT NULL,
    topic           text,

    CONSTRAINT room_pk PRIMARY KEY (id)
);

-- Table for holding active per-room per-user settings
CREATE TABLE lantern.room_members (
    user_id         bigint      NOT NULL,
    room_id         bigint      NOT NULL,

    -- if NULL, there is no difference between these and party_member perms
    -- full permissions can be computed from `(party_member.perms & !deny) | allow`
    allow           bigint, -- (user_allow | (role_allow & !user_deny))
    deny            bigint, -- (role_deny | user_deny)

    last_read       bigint,

    wallpaper_id    bigint,

    -- If NULL, there is no mute
    mute_expires    timestamptz,

    flags           int    NOT NULL DEFAULT 0,

    CONSTRAINT room_members_pk PRIMARY KEY (room_id, user_id)
);
COMMENT ON TABLE lantern.room_members IS 'Table for holding active per-room per-user settings.';
COMMENT ON COLUMN lantern.room_members.allow IS '`(user_allow | (role_allow & !user_deny))`';
COMMENT ON COLUMN lantern.room_members.deny IS '`(role_deny | user_deny)`';
COMMENT ON COLUMN lantern.room_members.mute_expires IS 'If NULL, there is no mute';

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
COMMENT ON TABLE lantern.files IS 'Backing file table for all attachments, avatars and so forth';
COMMENT ON COLUMN lantern.files.nonce IS 'Encryption Nonce';
COMMENT ON COLUMN lantern.files.size IS 'Size of file in bytes';
COMMENT ON COLUMN lantern.files.name IS 'Filename given at upload';
COMMENT ON COLUMN lantern.files.mime IS 'MIME type';
COMMENT ON COLUMN lantern.files.sha1 IS 'SHA-1 hash of completed file';
COMMENT ON COLUMN lantern.files.preview IS 'blurhash preview (first frame of video if video). this shouldn''t be too large, less than 128 bytes.';

CREATE TABLE lantern.user_assets (
    id          bigint      NOT NULL,

    -- original asset before processing
    file_id     bigint      NOT NULL,

    -- have one single blurhash preview for all versions of this asset
    preview     bytea,

    CONSTRAINT user_asset_pk PRIMARY KEY (id)
);
COMMENT ON COLUMN lantern.user_assets.file_id IS 'Original asset before processing';
COMMENT ON COLUMN lantern.user_assets.preview IS 'One single blurhash preview for all versions of this asset';

CREATE TABLE lantern.user_asset_files (
    asset_id    bigint      NOT NULL,
    file_id     bigint      NOT NULL,

    -- will contain info about file type and quality settings
    flags       smallint    NOT NULL,

    CONSTRAINT user_asset_files_pk PRIMARY KEY (asset_id, file_id)
);

-- Users can have multiple profiles, with one main profile where the `party_id` is NULL
CREATE TABLE lantern.profiles (
    user_id         bigint  NOT NULL,
    party_id        bigint,
    avatar_id       bigint,
    banner_id       bigint,
    bits            int NOT NULL DEFAULT 0,
    extra           int,
    nickname        text,
    custom_status   text,
    biography       text
);
COMMENT ON TABLE lantern.profiles IS 'Users can have multiple profiles, with one main profile where the `party_id` is NULL';

CREATE TABLE lantern.messages (
    -- Snowflake ID, contains created_at timestamp
    id          bigint      NOT NULL,
    user_id     bigint      NOT NULL,
    room_id     bigint      NOT NULL,
    thread_id   bigint,
    updated_at  timestamptz             DEFAULT now(),
    edited_at   timestamptz,
    kind        smallint    NOT NULL    DEFAULT 0,
    flags       smallint    NOT NULL    DEFAULT 0,
    content     text,

    -- take the top 6 bits of the smallint flags as a language code
    ts tsvector GENERATED ALWAYS AS (to_tsvector(lantern.to_language(flags >> 10), content)) STORED,

    pin_tags    bigint[],

    CONSTRAINT messages_pk PRIMARY KEY (id)
);
ALTER TABLE lantern.messages SET (toast_tuple_target = 128);

-- Message attachments association map
CREATE TABLE lantern.attachments (
    message_id  bigint      NOT NULL,
    file_id     bigint      NOT NULL,

    -- Flags are nullable to save 2-bytes per row in *most* cases
    flags       smallint,

    CONSTRAINT attachment_pk PRIMARY KEY (message_id, file_id)
);

CREATE TABLE lantern.emotes (
    id              bigint      NOT NULL,
    party_id        bigint,
    asset_id         bigint      NOT NULL,
    aspect_ratio    real        NOT NULL,
    flags           smallint    NOT NULL,
    name            text        NOT NULL,
    alt             text,

    CONSTRAINT emotes_pk PRIMARY KEY (id)
);

CREATE SEQUENCE lantern.emoji_id;

CREATE TABLE lantern.emojis (
    id          int         NOT NULL DEFAULT nextval('lantern.emoji_id'),

    -- like whether it supports skin tones
    flags       smallint    NOT NULL    DEFAULT 0,
    emoji       text        NOT NULL,
    description text                    DEFAULT NULL,
    aliases     text                    DEFAULT NULL,
    tags        text                    DEFAULT NULL,

    CONSTRAINT emojis_pk PRIMARY KEY (id)
);

ALTER SEQUENCE lantern.emoji_id OWNED BY lantern.emojis.id;

CREATE TABLE lantern.roles (
    id              bigint      NOT NULL,
    party_id        bigint      NOT NULL,
    avatar_id       bigint,
    -- Actually contains 3 16-bit fields
    permissions     bigint      NOT NULL    DEFAULT 0,
    color           integer,
    position        smallint    NOT NULL    DEFAULT 0,
    flags           smallint    NOT NULL    DEFAULT 0,
    name            text        NOT NULL,

    CONSTRAINT role_pk PRIMARY KEY (id)
);
COMMENT ON COLUMN lantern.roles.permissions IS 'Actually contains 3 16-bit fields';

-- Role/User association map
-- The party id can be found by joining with the role itself
CREATE TABLE lantern.role_members (
    role_id    bigint NOT NULL,
    user_id    bigint NOT NULL,

    CONSTRAINT role_member_pk PRIMARY KEY (role_id, user_id)
);

CREATE TABLE lantern.invite (
    id          bigint      NOT NULL,
    party_id    bigint      NOT NULL,
    user_id     bigint      NOT NULL,
    expires     timestamptz   NOT NULL,
    uses        int         NOT NULL    DEFAULT 0,
    max_uses    int         NOT NULL    DEFAULT 1,
    description text        NOT NULL,
    vanity      text,

    CONSTRAINT invite_pk PRIMARY KEY (id)
);

CREATE TABLE lantern.sessions (
    user_id bigint      NOT NULL,
    expires timestamptz NOT NULL,
    addr    inet        NOT NULL,
    token   bytea       NOT NULL
);

CREATE TABLE lantern.dms (
    user_id_a   bigint      NOT NULL,
    user_id_b   bigint      NOT NULL,
    room_id     bigint      NOT NULL,
    CONSTRAINT dm_pk PRIMARY KEY (user_id_a, user_id_b)
);

CREATE TABLE lantern.groups (
    id          bigint      NOT NULL,
    room_id     bigint      NOT NULL,

    CONSTRAINT group_pk PRIMARY KEY (id)
);

CREATE TABLE lantern.group_members (
    group_id    bigint      NOT NULL,
    user_id     bigint      NOT NULL,

    CONSTRAINT group_member_pk PRIMARY KEY (group_id, user_id)
);

-- CREATE TABLE lantern.room_users (
--     room_id     bigint      NOT NULL,
--     user_id     bigint      NOT NULL,

--     -- applicable for notifications
--     last_read   bigint,
--     -- applicable for slowmode
--     last_sent   bigint,
--     -- muted users cannot speak
--     muted       boolean,

--     CONSTRAINT room_users_pk PRIMARY KEY (room_id, user_id)
-- );

CREATE TABLE lantern.overwrites (
    room_id         bigint      NOT NULL,

    allow           bigint      NOT NULL    DEFAULT 0,
    deny            bigint      NOT NULL    DEFAULT 0,

    role_id         bigint,
    user_id         bigint
);

CREATE TABLE lantern.user_status (
    user_id         bigint      NOT NULL,
    updated         timestamptz NOT NULL DEFAULT now(),
    active          smallint    NOT NULL DEFAULT 0,

    CONSTRAINT user_status_pk PRIMARY KEY (user_id)
);

CREATE TABLE lantern.reactions (
    msg_id      bigint      NOT NULL,
    emote_id    bigint,

    reacted     timestamptz NOT NULL DEFAULT now(),

    -- placed here due to alignment of int4
    emoji_id    integer,

    user_ids    bigint[]    NOT NULL
);

CREATE TABLE lantern.mentions (
    msg_id      bigint NOT NULL,

    user_id     bigint,
    role_id     bigint,
    room_id     bigint
);

CREATE TABLE lantern.friends (
    user_a_id   bigint      NOT NULL,
    user_b_id   bigint      NOT NULL,
    updated_at  timestamptz NOT NULL DEFAULT now(),
    flags       smallint    NOT NULL DEFAULT 0,
    note_a      text,
    note_b      text
);

CREATE TABLE lantern.user_presence (
    user_id     bigint      NOT NULL,
    -- Connection ID, only really seen on the server layer
    conn_id     bigint      NOT NULL,
    updated_at  timestamptz NOT NULL DEFAULT now(),
    flags       smallint    NOT NULL,
    activity    jsonb,

    CONSTRAINT presence_pk PRIMARY KEY (user_id, conn_id)
);

CREATE TABLE IF NOT EXISTS lantern.party_bans (
    party_id    bigint      NOT NULL,
    user_id     bigint      NOT NULL,

    banned_at   timestamptz NOT NULL DEFAULT now(),
    reason      text,

    CONSTRAINT party_bans_pk PRIMARY KEY (party_id, user_id)
);

CREATE TABLE IF NOT EXISTS lantern.user_blocks (
    user_id     bigint      NOT NULL,
    block_id    bigint      NOT NULL,

    blocked_at  timestamptz NOT NULL DEFAULT now(),

    CONSTRAINT user_blocks_pk PRIMARY KEY (user_id, block_id)
);

CREATE TABLE IF NOT EXISTS lantern.embed_cache (
    id      bigint  NOT NULL,
    url     text    NOT NULL
);

CREATE TABLE IF NOT EXISTS lantern.threads (
    id          bigint      NOT NULL,
    -- The first message that started the thread
    parent_id   bigint      NOT NULL,

    flags       smallint    NOT NULL DEFAULT 0,

    CONSTRAINT thread_pk PRIMARY KEY (id)
);

CREATE TABLE lantern.pin_tags (
    id          bigint      NOT NULL,

    icon_id     bigint, -- emote id for icon

    -- might include color
    flags       int         NOT NULL DEFAULT 0,

    name        text        NOT NULL,
    description text,

    CONSTRAINT pin_tags_pk PRIMARY KEY (id)
);

----------------------------------------
----------- FOREIGN KEYS ---------------
----------------------------------------

ALTER TABLE lantern.event_log ADD CONSTRAINT party_fk FOREIGN KEY (party_id)
    REFERENCES lantern.party (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;

ALTER TABLE lantern.event_log ADD CONSTRAINT room_fk FOREIGN KEY (room_id)
    REFERENCES lantern.rooms (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;

ALTER TABLE lantern.user_tokens ADD CONSTRAINT user_fk FOREIGN KEY (user_id)
    REFERENCES lantern.users (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;

ALTER TABLE lantern.party ADD CONSTRAINT owner_fk FOREIGN KEY (owner_id)
    REFERENCES lantern.users (id) MATCH FULL
    ON DELETE RESTRICT ON UPDATE CASCADE; -- Don't allow users to delete accounts if they own parties

ALTER TABLE lantern.party_member ADD CONSTRAINT party_fk FOREIGN KEY (party_id)
    REFERENCES lantern.party (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE; -- When a party is deleted cascade to delete memberships

ALTER TABLE lantern.party_member ADD CONSTRAINT member_fk FOREIGN KEY (user_id)
    REFERENCES lantern.users (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE; -- When a user is deleted cascade to delete their membership

ALTER TABLE lantern.rooms ADD CONSTRAINT party_fk FOREIGN KEY (party_id)
    REFERENCES lantern.party (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE; -- Delete rooms if party is deleted

ALTER TABLE lantern.rooms ADD CONSTRAINT parent_fk FOREIGN KEY (parent_id)
    REFERENCES lantern.rooms (id) MATCH FULL
    ON DELETE SET NULL ON UPDATE CASCADE;

ALTER TABLE lantern.party ADD CONSTRAINT default_room_fk FOREIGN KEY (default_room)
    REFERENCES lantern.rooms (id) MATCH FULL
    ON DELETE RESTRICT ON UPDATE CASCADE -- don't allow deleting default room
    DEFERRABLE INITIALLY DEFERRED;

ALTER TABLE lantern.room_members ADD CONSTRAINT room_fk FOREIGN KEY (room_id)
    REFERENCES lantern.rooms (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;

ALTER TABLE lantern.room_members ADD CONSTRAINT user_fk FOREIGN KEY (user_id)
    REFERENCES lantern.users (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;

ALTER TABLE lantern.room_members ADD CONSTRAINT wall_fk FOREIGN KEY (wallpaper_id)
    REFERENCES lantern.files (id) MATCH FULL
    ON DELETE SET NULL ON UPDATE CASCADE;

ALTER TABLE lantern.files ADD CONSTRAINT user_fk FOREIGN KEY (user_id)
    REFERENCES lantern.users (id) MATCH FULL
    ON DELETE RESTRICT ON UPDATE CASCADE;

ALTER TABLE lantern.user_assets ADD CONSTRAINT file_id_fk FOREIGN KEY (file_id)
    REFERENCES lantern.files (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;

ALTER TABLE lantern.user_asset_files ADD CONSTRAINT asset_id_fk FOREIGN KEY (asset_id)
    REFERENCES lantern.user_assets (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;

ALTER TABLE lantern.user_asset_files ADD CONSTRAINT file_id_fk FOREIGN KEY (file_id)
    REFERENCES lantern.files (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;

ALTER TABLE lantern.party ADD CONSTRAINT avatar_fk FOREIGN KEY (avatar_id)
    REFERENCES lantern.user_assets (id) MATCH FULL
    ON DELETE SET NULL ON UPDATE CASCADE; -- IMPORTANT: Only set NULL when deleting avatars, DO NOT CASCADE

ALTER TABLE lantern.party ADD CONSTRAINT banner_fk FOREIGN KEY (banner_id)
    REFERENCES lantern.user_assets (id) MATCH FULL
    ON DELETE SET NULL ON UPDATE CASCADE; -- IMPORTANT: Only set NULL when deleting avatars, DO NOT CASCADE

ALTER TABLE lantern.rooms ADD CONSTRAINT avatar_fk FOREIGN KEY (avatar_id)
    REFERENCES lantern.user_assets (id) MATCH FULL
    ON DELETE SET NULL ON UPDATE CASCADE; -- IMPORTANT: Only set NULL when deleting avatars, DO NOT CASCADE

ALTER TABLE lantern.profiles ADD CONSTRAINT user_fk FOREIGN KEY(user_id)
    REFERENCES lantern.users (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;

ALTER TABLE lantern.profiles ADD CONSTRAINT party_fk FOREIGN KEY(party_id)
    REFERENCES lantern.party (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;

ALTER TABLE lantern.profiles ADD CONSTRAINT avatar_fk FOREIGN KEY(avatar_id)
    REFERENCES lantern.user_assets (id) MATCH FULL
    ON DELETE SET NULL ON UPDATE CASCADE;

ALTER TABLE lantern.profiles ADD CONSTRAINT banner_fk FOREIGN KEY(banner_id)
    REFERENCES lantern.user_assets (id) MATCH FULL
    ON DELETE SET NULL ON UPDATE CASCADE;

ALTER TABLE lantern.messages ADD CONSTRAINT room_fk FOREIGN KEY (room_id)
    REFERENCES lantern.rooms (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE; -- If room is deleted, delete all messages in room

ALTER TABLE lantern.messages ADD CONSTRAINT user_fk FOREIGN KEY (user_id)
    REFERENCES lantern.users (id) MATCH FULL
    ON DELETE SET NULL ON UPDATE CASCADE; -- If user is deleted, just set to NULL

ALTER TABLE lantern.attachments ADD CONSTRAINT file_fk FOREIGN KEY (file_id)
    REFERENCES lantern.files (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE; -- On file deletion, delete attachment entry

ALTER TABLE lantern.attachments ADD CONSTRAINT message_fk FOREIGN KEY (message_id)
    REFERENCES lantern.messages (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE; -- Delete attachments on REAL message deletion

ALTER TABLE lantern.emotes ADD CONSTRAINT party_fk FOREIGN KEY (party_id)
    REFERENCES lantern.party (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE; -- Delete emotes on party deletion

ALTER TABLE lantern.emotes ADD CONSTRAINT asset_fk FOREIGN KEY (asset_id)
    REFERENCES lantern.user_assets (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;

ALTER TABLE lantern.roles ADD CONSTRAINT party_fk FOREIGN KEY (party_id)
    REFERENCES lantern.party (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;

ALTER TABLE lantern.roles ADD CONSTRAINT avatar_fk FOREIGN KEY (avatar_id)
    REFERENCES lantern.user_assets (id) MATCH FULL
    ON DELETE SET NULL ON UPDATE CASCADE;

ALTER TABLE lantern.role_members ADD CONSTRAINT role_fk FOREIGN KEY (role_id)
    REFERENCES lantern.roles (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;

ALTER TABLE lantern.role_members ADD CONSTRAINT user_fk FOREIGN KEY (user_id)
    REFERENCES lantern.users (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;

ALTER TABLE lantern.invite ADD CONSTRAINT party_fk FOREIGN KEY (party_id)
    REFERENCES lantern.party (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;

ALTER TABLE lantern.invite ADD CONSTRAINT user_fk FOREIGN KEY (user_id)
    REFERENCES lantern.users (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;

ALTER TABLE lantern.party_member ADD CONSTRAINT invite_fk FOREIGN KEY (invite_id)
    REFERENCES lantern.invite (id) MATCH FULL
    ON DELETE SET NULL ON UPDATE CASCADE;

ALTER TABLE lantern.sessions ADD CONSTRAINT user_fk FOREIGN KEY (user_id)
    REFERENCES lantern.users (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;

ALTER TABLE lantern.dms ADD CONSTRAINT user_id_a_fk FOREIGN KEY (user_id_a)
    REFERENCES lantern.users (id) MATCH FULL
    ON DELETE NO ACTION ON UPDATE CASCADE; -- Leave DM open on user deletion?

ALTER TABLE lantern.dms ADD CONSTRAINT user_id_b_fk FOREIGN KEY (user_id_b)
    REFERENCES lantern.users (id) MATCH FULL
    ON DELETE NO ACTION ON UPDATE CASCADE; -- Leave DM open on user deletion?

ALTER TABLE lantern.dms ADD CONSTRAINT room_id_fk FOREIGN KEY (room_id)
    REFERENCES lantern.rooms (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE; -- delete DM if channel is deleted?

ALTER TABLE lantern.group_members ADD CONSTRAINT group_id_fk FOREIGN KEY (group_id)
    REFERENCES lantern.groups (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE; -- Delete members if whole group is deleted

ALTER TABLE lantern.group_members ADD CONSTRAINT user_id_fk FOREIGN KEY (user_id)
    REFERENCES lantern.users (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE; -- Delete member if user is deleted

-- ALTER TABLE lantern.room_users ADD CONSTRAINT room_id_fk FOREIGN KEY (room_id)
--     REFERENCES lantern.rooms (id) MATCH FULL
--     ON DELETE CASCADE ON UPDATE CASCADE;

-- ALTER TABLE lantern.room_users ADD CONSTRAINT user_id_fk FOREIGN KEY (user_id)
--     REFERENCES lantern.users (id) MATCH FULL
--     ON DELETE CASCADE ON UPDATE CASCADE;

-- -- On delete, don't update this as the stored id still contains the timestamp
-- ALTER TABLE lantern.room_users ADD CONSTRAINT last_read_fk FOREIGN KEY (last_read)
--     REFERENCES lantern.messages (id) MATCH FULL
--     ON DELETE NO ACTION ON UPDATE CASCADE;

-- -- On delete, don't update this as the stored id still contains the timestamp
-- ALTER TABLE lantern.room_users ADD CONSTRAINT last_sent_fk FOREIGN KEY (last_sent)
--     REFERENCES lantern.messages (id) MATCH FULL
--     ON DELETE NO ACTION ON UPDATE CASCADE;

ALTER TABLE lantern.overwrites ADD CONSTRAINT room_id_fk FOREIGN KEY (room_id)
    REFERENCES lantern.rooms (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;

ALTER TABLE lantern.overwrites ADD CONSTRAINT role_id_fk FOREIGN KEY (role_id)
    REFERENCES lantern.roles (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;

ALTER TABLE lantern.overwrites ADD CONSTRAINT user_id_fk FOREIGN KEY (user_id)
    REFERENCES lantern.users (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;

ALTER TABLE lantern.reactions ADD CONSTRAINT msg_id_fk FOREIGN KEY (msg_id)
    REFERENCES lantern.messages (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;

ALTER TABLE lantern.reactions ADD CONSTRAINT emote_id_fk FOREIGN KEY (emote_id)
    REFERENCES lantern.emotes (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;

ALTER TABLE lantern.reactions ADD CONSTRAINT emoji_id_fk FOREIGN KEY (emoji_id)
    REFERENCES lantern.emojis (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;

ALTER TABLE lantern.user_status ADD CONSTRAINT user_fk FOREIGN KEY(user_id)
    REFERENCES lantern.users (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;

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

ALTER TABLE lantern.friends ADD CONSTRAINT user_a_fk FOREIGN KEY(user_a_id)
    REFERENCES lantern.users (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;

ALTER TABLE lantern.friends ADD CONSTRAINT user_b_fk FOREIGN KEY(user_b_id)
    REFERENCES lantern.users (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;

ALTER TABLE lantern.user_presence ADD CONSTRAINT user_fk FOREIGN KEY(user_id)
    REFERENCES lantern.users (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;

ALTER TABLE lantern.party_bans ADD CONSTRAINT party_fk FOREIGN KEY (party_id)
    REFERENCES lantern.party (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;

ALTER TABLE lantern.party_bans ADD CONSTRAINT user_fk FOREIGN KEY (user_id)
    REFERENCES lantern.users (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;

ALTER TABLE lantern.user_blocks ADD CONSTRAINT user_fk FOREIGN KEY (user_id)
    REFERENCES lantern.users (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;

ALTER TABLE lantern.user_blocks ADD CONSTRAINT block_fk FOREIGN KEY (block_id)
    REFERENCES lantern.users (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;

ALTER TABLE lantern.messages ADD CONSTRAINT thread_fk FOREIGN KEY (thread_id)
    REFERENCES lantern.threads (id) MATCH FULL
    ON DELETE SET NULL ON UPDATE CASCADE;

-- Don't allow parent messages to be deleted, they must be handled specially
ALTER TABLE lantern.threads ADD CONSTRAINT message_fk FOREIGN KEY (parent_id)
    REFERENCES lantern.messages (id) MATCH FULL
    ON DELETE RESTRICT ON UPDATE CASCADE;

ALTER TABLE lantern.pin_tags ADD CONSTRAINT icon_fk FOREIGN KEY (icon_id)
    REFERENCES lantern.emotes (id) MATCH FULL
    ON DELETE SET NULL ON UPDATE CASCADE;


----------------------------------------
------------ CONSTRAINTS ---------------
----------------------------------------

-- per-user, their parties must be sorted with unique positions
ALTER TABLE lantern.party_member ADD CONSTRAINT unique_party_position
    UNIQUE(user_id, position) DEFERRABLE INITIALLY DEFERRED;

-- per-party, their rooms must be sorted with unique positions
ALTER TABLE lantern.rooms ADD CONSTRAINT unique_room_position
    UNIQUE(party_id, position) DEFERRABLE INITIALLY DEFERRED;

-- Each attachment has a unique file
ALTER TABLE lantern.attachments ADD CONSTRAINT attachment_uq
    UNIQUE (file_id);

ALTER TABLE lantern.roles ADD CONSTRAINT unique_role_position
    UNIQUE(party_id, position) DEFERRABLE INITIALLY DEFERRED;

ALTER TABLE lantern.reactions ADD CONSTRAINT uq_msg_emote
    UNIQUE(msg_id, emote_id);

ALTER TABLE lantern.reactions ADD CONSTRAINT uq_msg_emoji
    UNIQUE(msg_id, emoji_id);

-- assert that each mention as AT MOST AND AT LEAST 1 valid ID
ALTER TABLE lantern.mentions ADD CONSTRAINT check_all CHECK (
    1 = (user_id IS NOT NULL)::int4 + (role_id IS NOT NULL)::int4 + (room_id IS NOT NULL)::int4
);

-- asset that at least address or network is not null
ALTER TABLE lantern.ip_bans ADD CONSTRAINT addr_check CHECK (
    address IS NOT NULL OR network IS NOT NULL
);

-- Messages can only be the parent of a single thread
ALTER TABLE lantern.threads ADD CONSTRAINT parent_uq UNIQUE (parent_id);

-- ensure proper ordering of friend columns
ALTER TABLE lantern.friends ADD CONSTRAINT ch_friend_order CHECK (user_a_id < user_b_id);

-- user cannot block themselves
ALTER TABLE lantern.user_blocks ADD CONSTRAINT ch_user_blocks CHECK (user_id <> block_id);

----------------------------------------
------- CONSTRAINT-LIKE INDICES --------
----------------------------------------

-- Fast lookup of users via `username#0000`, and enforce that unique combination
CREATE UNIQUE INDEX user_username_discriminator_idx ON lantern.users
    USING btree (username, discriminator);

CREATE UNIQUE INDEX user_email_idx ON lantern.users
    USING btree(email);

CREATE UNIQUE INDEX user_freelist_username_discriminator_idx ON lantern.user_freelist
    USING btree (username, discriminator);

-- ensure there can only be one profile per user per party (or no party)
CREATE UNIQUE INDEX profiles_user_party_idx ON lantern.profiles
    USING btree(user_id, COALESCE(party_id, 1));

CREATE UNIQUE INDEX attachment_file_idx ON lantern.attachments
    USING btree(file_id);

CREATE UNIQUE INDEX emote_name_idx ON lantern.emotes
    USING btree (party_id, name);

CREATE UNIQUE INDEX invite_vanity_idx ON lantern.invite
    USING btree(vanity) WHERE vanity IS NOT NULL;

CREATE UNIQUE INDEX overwrites_room_role_idx ON lantern.overwrites
    USING btree(room_id, role_id) WHERE role_id IS NOT NULL;
CREATE UNIQUE INDEX overwrites_room_user_idx ON lantern.overwrites
    USING btree(room_id, user_id) WHERE user_id IS NOT NULL;

CREATE UNIQUE INDEX reactions_emote_idx ON lantern.reactions
    USING btree(msg_id, emote_id) WHERE emote_id IS NOT NULL;
CREATE UNIQUE INDEX reactions_emoji_idx ON lantern.reactions
    USING btree(msg_id, emoji_id) WHERE emoji_id IS NOT NULL;

CREATE UNIQUE INDEX emoji_idx ON lantern.emojis
    USING btree(emoji);

CREATE UNIQUE INDEX friend_idx ON lantern.friends
    USING btree(user_a_id, user_b_id);

CREATE UNIQUE INDEX user_blocks_idx ON lantern.user_blocks
    USING btree(user_id, block_id);

----------------------------------------
-------------- INDICES -----------------
----------------------------------------

CREATE INDEX event_log_counter_idx          ON lantern.event_log        USING btree(counter);

CREATE INDEX user_username_idx              ON lantern.users            USING btree(username);
CREATE INDEX user_freelist_username_idx     ON lantern.user_freelist    USING btree(username);

-- tokens are random bits, so hash-based lookup is fine
CREATE INDEX user_tokens_token_idx          ON lantern.user_tokens      USING hash(token);
CREATE INDEX user_tokens_expires_idx        ON lantern.user_tokens      USING btree(expires);
CREATE INDEX party_name_idx                 ON lantern.party            USING btree(name);
CREATE INDEX party_member_user_idx          ON lantern.party_member     USING btree(user_id);
CREATE INDEX room_name_idx                  ON lantern.rooms            USING btree(name);
CREATE INDEX room_avatar_idx                ON lantern.rooms            USING btree(avatar_id) WHERE avatar_id IS NOT NULL;
CREATE INDEX file_idx                       ON lantern.files            USING btree(user_id, id)        INCLUDE (size);
CREATE INDEX user_asset_original_file_idx   ON lantern.user_assets      USING btree(file_id);

-- TODO: Is this even necessary with such a simple table? The index itself has the same information as the actual table
CREATE INDEX user_asset_file_idx            ON lantern.user_asset_files USING btree(asset_id, file_id)  INCLUDE (flags);

-- Since id is a snowflake, it's always sorted by time
-- so index with btree for the times when we need to fetch by timestamps
CREATE INDEX msg_id_idx                     ON lantern.messages         USING btree(id);

-- mutually exclusive indexes
CREATE INDEX msg_dl_idx                     ON lantern.messages         USING btree(room_id, id) WHERE flags & 1 = 1;
CREATE INDEX msg_nd_idx                     ON lantern.messages         USING btree(room_id, id) WHERE flags & 1 = 0;


CREATE INDEX msg_ts_idx                     ON lantern.messages         USING GIN (ts);
CREATE INDEX message_pin_tag_idx            ON lantern.messages         USING GIN (pin_tags) WHERE pin_tags IS NOT NULL;

CREATE INDEX attachment_msg_idx             ON lantern.attachments      USING btree(message_id); -- INCLUDE(flags) ?

CREATE INDEX emote_party_idx                ON lantern.emotes           USING btree(party_id);
CREATE INDEX role_party_idx                 ON lantern.roles            USING btree(party_id);

-- tokens are random bits, so hash-based lookup is fine
CREATE INDEX session_token_idx              ON lantern.sessions         USING hash(token);
CREATE INDEX session_expires_idx            ON lantern.sessions         USING btree(expires);

CREATE INDEX dm_user_a_idx                  ON lantern.dms              USING btree(user_id_a);
CREATE INDEX dm_user_b_idx                  ON lantern.dms              USING btree(user_id_b);
CREATE INDEX group_member_id_idx            ON lantern.group_members    USING btree(group_id);
CREATE INDEX group_member_user_idx          ON lantern.group_members    USING btree(user_id);

CREATE INDEX overwrites_room_idx            ON lantern.overwrites       USING btree(room_id);
CREATE INDEX user_status_user_idx           ON lantern.user_status      USING btree(user_id);
CREATE INDEX user_status_time_idx           ON lantern.user_status      USING btree(updated);

CREATE INDEX reaction_msg_idx               ON lantern.reactions        USING btree(msg_id);

-- allow to find and sort by msg id
CREATE INDEX mention_msg_idx                ON lantern.mentions         USING btree(msg_id);

-- allow a user to search for their own mentions
CREATE INDEX mention_user_idx               ON lantern.mentions         USING btree (user_id) WHERE user_id IS NOT NULL;
CREATE INDEX mention_role_idx               ON lantern.mentions         USING btree (role_id) WHERE role_id IS NOT NULL;

CREATE INDEX rate_limit_idx                 ON lantern.rate_limits      USING btree(addr);
CREATE INDEX ip_bans_address_idx            ON lantern.ip_bans          USING btree(address) WHERE address IS NOT NULL;
CREATE INDEX ip_bans_network_idx            ON lantern.ip_bans          USING GIST(network inet_ops) WHERE network IS NOT NULL;
CREATE INDEX friend_a_idx                   ON lantern.friends          USING btree(user_a_id);
CREATE INDEX friend_b_idx                   ON lantern.friends          USING btree(user_b_id);
CREATE INDEX user_presence_conn_idx         ON lantern.user_presence    USING btree(conn_id);
CREATE INDEX user_presence_idx              ON lantern.user_presence    USING btree(user_id, updated_at);

CREATE INDEX user_block_user_idx            ON lantern.user_blocks      USING btree(user_id);
CREATE INDEX metrics_ts_idx                 ON lantern.metrics          USING btree(ts);

CREATE INDEX room_member_wallpaper_idx      ON lantern.room_members     USING btree(wallpaper_id) WHERE wallpaper_id IS NOT NULL;

----------------------------------------
----------- INITIAL VALUES -------------
----------------------------------------

-- setup event_log_last_notification so queries can return a value
INSERT INTO lantern.event_log_last_notification DEFAULT VALUES;

-- Create SYSTEM user for sending system messages
INSERT INTO lantern.users (id, dob, flags, username, discriminator, email, passhash)
    VALUES (1, date '1970-01-01', 256, 'SYSTEM', 0, '', '') ON CONFLICT DO NOTHING;

----------------------------------------
-------------- TRIGGERS ----------------
----------------------------------------

-- Trigger function for rate-limited notifications
CREATE OR REPLACE FUNCTION lantern.ev_notify_trigger()
RETURNS trigger
LANGUAGE plpgsql AS
$$
DECLARE
    _last_notif timestamptz;
    _max_interval interval;
    _now timestamptz := now();
BEGIN
    SELECT
        last_notif, max_interval
    INTO
        _last_notif, _max_interval
    FROM lantern.event_log_last_notification FETCH FIRST ROW ONLY;

    IF (_now - _last_notif) >= _max_interval THEN
        PERFORM pg_notify('event_log', (NEW.id)::text);
        UPDATE lantern.event_log_last_notification SET
            last_notif = _now;
    END IF;
    RETURN NEW;
END
$$;

CREATE TRIGGER event_log_notify AFTER INSERT ON lantern.event_log
FOR EACH ROW EXECUTE FUNCTION lantern.ev_notify_trigger();

--

CREATE OR REPLACE FUNCTION lantern.msg_trigger()
RETURNS trigger
LANGUAGE plpgsql AS
$$
BEGIN
    INSERT INTO lantern.event_log (code, id, party_id)
    SELECT
        -- when old was not deleted, and new is deleted
        CASE WHEN (OLD.flags & 1 = 0) AND (NEW.flags & 1 != 0)
                THEN 'message_delete'::lantern.event_code

             WHEN TG_OP = 'INSERT'
                THEN 'message_create'::lantern.event_code

             WHEN TG_OP = 'UPDATE' AND (NEW.flags & 1 = 0)
                THEN 'message_update'::lantern.event_code
        END,
        NEW.id,
        (SELECT party_id FROM lantern.rooms WHERE rooms.id = NEW.room_id);

    RETURN NEW;
END
$$;

CREATE TRIGGER message_event AFTER UPDATE OR INSERT ON lantern.messages
FOR EACH ROW EXECUTE FUNCTION lantern.msg_trigger();

--

CREATE OR REPLACE FUNCTION lantern.presence_trigger()
RETURNS trigger
LANGUAGE plpgsql AS
$$
BEGIN
    INSERT INTO lantern.event_log (code, id) VALUES (
        'presence_updated'::lantern.event_code,
        CASE TG_OP WHEN 'DELETE' THEN OLD.user_id
                                 ELSE NEW.user_id
        END
    );

    -- if a new presence includes the online flag
    IF TG_OP != 'DELETE' AND (NEW.flags & (1 << 2)) = (1 << 2) THEN
        UPDATE lantern.users SET last_active = now() WHERE id = NEW.user_id;
    END IF;

    RETURN NEW;
END
$$;

CREATE TRIGGER presence_update AFTER INSERT OR UPDATE OR DELETE ON lantern.user_presence
FOR EACH ROW EXECUTE FUNCTION lantern.presence_trigger();

--

-- Ban lifecycle:
-- Start out without a ban
-- Banned, emit memebr_ban and app code also emits member_left
-- Member is no longer visible in party, cannot rejoin
-- Member unbanned, delete member row and emit member_unban

CREATE OR REPLACE FUNCTION lantern.on_member_insert()
RETURNS trigger
LANGUAGE plpgsql AS
$$
BEGIN
    INSERT INTO lantern.event_log (code, id, party_id)
    VALUES (
        'member_joined'::lantern.event_code,
        NEW.user_id,
        NEW.party_id
    );
    RETURN NEW;
END
$$;

CREATE OR REPLACE FUNCTION lantern.on_member_delete()
RETURNS trigger
LANGUAGE plpgsql AS
$$
BEGIN
    INSERT INTO lantern.event_log (code, id, party_id)
    SELECT
        CASE
            -- Deleting a member entry when unbanning signifies the ban has been lifted
            -- but they must rejoin manually
            WHEN ((OLD.flags & 1 = 1))
                THEN 'member_unban'::lantern.event_code
            ELSE 'member_left'::lantern.event_code
        END,
        OLD.user_id,
        OLD.party_id;
    RETURN NEW;
END
$$;

CREATE OR REPLACE FUNCTION lantern.on_member_update()
RETURNS trigger
LANGUAGE plpgsql AS
$$
BEGIN
    -- NOTE: If perms was updated via another trigger, then the condition on
    -- member_update_event prevents this trigger from being called in the first place
    IF OLD.perms != NEW.perms THEN
        -- do nothing when cached permissions change
        -- NOTE: When updating perms, make sure a `WHERE perms != new_perms` is given on the UPDATE to avoid triggering
        RETURN NEW;
    ELSEIF OLD.position != NEW.position THEN
        -- Force a self-update to refresh party positions
        INSERT INTO lantern.event_log(code, id, party_id)
        VALUES('self_updated'::lantern.event_code, NEW.user_id, NEW.party_id);
    ELSEIF (OLD.flags != NEW.flags) THEN
        INSERT INTO lantern.event_log (code, id, party_id)
        SELECT
            CASE
                WHEN ((OLD.flags & 1 = 0)) AND ((NEW.flags & 1 = 1))
                    THEN 'member_ban'::lantern.event_code
                ELSE 'member_updated'::lantern.event_code
            END,
            NEW.user_id,
            NEW.party_id;
    END IF;

    RETURN NEW;
END
$$;

CREATE TRIGGER member_insert_event AFTER INSERT ON lantern.party_member
FOR EACH ROW EXECUTE FUNCTION lantern.on_member_insert();

CREATE TRIGGER member_update_event AFTER UPDATE ON lantern.party_member
FOR EACH ROW WHEN (pg_trigger_depth() = 0)
EXECUTE FUNCTION lantern.on_member_update();

CREATE TRIGGER member_delete_event AFTER DELETE ON lantern.party_member
FOR EACH ROW EXECUTE FUNCTION lantern.on_member_delete();

--

-- Updating role_members should trigger a member_updated event
CREATE OR REPLACE FUNCTION lantern.role_member_trigger()
RETURNS trigger
LANGUAGE plpgsql AS
$$
BEGIN
    INSERT INTO lantern.event_log (code, id, party_id)
    SELECT 'member_updated'::lantern.event_code,
        COALESCE(OLD.user_id, NEW.user_id),
        roles.party_id
    FROM lantern.roles WHERE roles.id = COALESCE(OLD.role_id, NEW.role_id);

    RETURN NEW;
END
$$;

CREATE TRIGGER role_member_event AFTER UPDATE OR INSERT OR DELETE ON lantern.role_members
FOR EACH ROW EXECUTE FUNCTION lantern.role_member_trigger();

--

-- emit role_deleted/created/updated events
CREATE OR REPLACE FUNCTION lantern.role_trigger()
RETURNS trigger
LANGUAGE plpgsql AS
$$
BEGIN

    IF TG_OP = 'DELETE' THEN
        INSERT INTO lantern.event_log (code, id, party_id)
        VALUES ('role_deleted'::lantern.event_code, OLD.id, OLD.party_id);
    ELSE
        INSERT INTO lantern.event_log(code, id, party_id)
        SELECT
            CASE
                WHEN TG_OP = 'INSERT'
                    THEN 'role_created'::lantern.event_code
                    ELSE 'role_updated'::lantern.event_code
            END,
            NEW.id,
            NEW.party_id;

    END IF;

    RETURN NEW;
END
$$;

CREATE TRIGGER role_event AFTER UPDATE OR INSERT OR DELETE ON lantern.roles
FOR EACH ROW EXECUTE FUNCTION lantern.role_trigger();

--

-- emit 'self_updated' or 'user_updated' events
-- NOTE: Should be kept in-sync with user fields
CREATE OR REPLACE FUNCTION lantern.user_trigger()
RETURNS trigger
LANGUAGE plpgsql AS
$$
BEGIN
    IF
        OLD.preferences IS DISTINCT FROM NEW.preferences OR
        OLD.dob != NEW.dob OR
        OLD.email != NEW.email OR
        OLD.flags != NEW.flags -- TODO: only compare public fields?
    THEN
        -- self event when changing private fields
        INSERT INTO lantern.event_log(code, id)
        VALUES ('self_updated'::lantern.event_code, NEW.id);
    ELSIF
        OLD.username != NEW.username OR
        OLD.deleted_at IS DISTINCT FROM NEW.deleted_at
    THEN
        -- user event
        INSERT INTO lantern.event_log(code, id)
        VALUES ('user_updated'::lantern.event_code, NEW.id);
    END IF;

    -- ignore any other fields

    RETURN NEW;
END
$$;

CREATE TRIGGER user_event AFTER UPDATE ON lantern.users
FOR EACH ROW EXECUTE FUNCTION lantern.user_trigger();

--

CREATE OR REPLACE FUNCTION lantern.profile_trigger()
RETURNS trigger
LANGUAGE plpgsql AS
$$
BEGIN
    -- If updating or any values we care about for profile_updated are the same
    IF TG_OP != 'UPDATE' OR (
        OLD.bits != NEW.bits OR
        (OLD.extra IS DISTINCT FROM NEW.extra) OR
        (OLD.custom_status IS DISTINCT FROM NEW.custom_status) OR
        (OLD.nickname IS DISTINCT FROM NEW.nickname) OR
        (OLD.avatar_id IS DISTINCT FROM NEW.avatar_id)
    ) THEN
        INSERT INTO lantern.event_log (id, party_id, code)
        VALUES (
            COALESCE(OLD.user_id, NEW.user_id),
            COALESCE(OLD.party_id, NEW.party_id),
            'profile_updated'::lantern.event_code
        );
    END IF;

    RETURN NEW;
END
$$;

CREATE TRIGGER profile_event AFTER UPDATE OR INSERT OR DELETE ON lantern.profiles
FOR EACH ROW EXECUTE FUNCTION lantern.profile_trigger();

--

-- When a party_member row is deleted, also delete their per-party profile override entry
CREATE OR REPLACE FUNCTION lantern.party_member_delete_profile_trigger()
RETURNS trigger
LANGUAGE plpgsql AS
$$
BEGIN
    DELETE FROM lantern.profiles WHERE user_id = OLD.user_id AND party_id = OLD.party_id;
    RETURN NEW;
END
$$;

CREATE TRIGGER party_member_delete_profile_event AFTER DELETE ON lantern.party_member
FOR EACH ROW EXECUTE FUNCTION lantern.party_member_delete_profile_trigger();

--

-- ensure referenced pin_tags are removed from the denormalized array.
-- this acts like a foreign key with ON DELETE CASCADE
CREATE OR REPLACE FUNCTION lantern.pin_tag_delete_trigger()
RETURNS trigger
LANGUAGE plpgsql AS
$$
BEGIN
    IF TG_OP = 'DELETE' THEN
        UPDATE lantern.messages SET pin_tags = array_remove(pin_tags, OLD.id)
            WHERE pin_tags @> ARRAY[OLD.id];
    END IF;

    RETURN NEW;
END
$$;

CREATE TRIGGER pin_tag_delete_event AFTER DELETE ON lantern.pin_tags
FOR EACH ROW EXECUTE FUNCTION lantern.pin_tag_delete_trigger();

----------------------------------------
------------ PERM TRIGGERS -------------
----------------------------------------

-- When a role updates, the change should cascade down each user's
-- base permissiona and their specific room permissions

CREATE OR REPLACE FUNCTION lantern.on_role_update_trigger()
RETURNS trigger
LANGUAGE plpgsql AS
$$
DECLARE
    _party_id bigint;
    _role_id bigint;
BEGIN
    IF OLD.permissions = NEW.permissions THEN
        RETURN NEW;
    END IF;

    SELECT
        COALESCE(NEW.party_id, OLD.party_id),
        COALESCE(NEW.role_id, OLD.role_id)
    INTO _party_id, _role_id;

    -- Handle @everyone special case
    IF NEW.id = NEW.party_id THEN
        WITH perms AS (
            SELECT party_member.user_id, bit_or(roles.permissions) AS perms
            FROM lantern.party_member
            INNER JOIN lantern.roles ON roles.id = party_member.party_id AND roles.party_id = party_member.party_id
            GROUP BY party_member.user_id
        )
        UPDATE party_member SET perms = perms.perms
        FROM perms WHERE party_member.user_id = perms.user_id
        AND party_member.party_id = _party_id
        AND party_member.perms != perms.perms;
    ELSE
        WITH members_to_update AS (
            -- get a list of members relevant to this role
            SELECT role_members.user_id FROM role_members WHERE role_members.role_id = _role_id
        ), perms AS (
            -- compute base permissions for each user
            SELECT
                m.user_id, bit_or(roles.permissions) AS perms
            FROM role_members
            -- join with roles to get roles.permissions
            INNER JOIN roles ON roles.id = role_members.role_id OR roles.id = roles.party_id
            -- join with m to limit members updated
            INNER JOIN members_to_update m ON role_members.user_id = m.user_id
            GROUP BY m.user_id
        )
        -- apply updated base permissions
        UPDATE lantern.party_member SET perms = perms.perms
        FROM perms WHERE party_member.user_id = perms.user_id
        AND party_member.party_id = _party_id
        -- but don't modify if unchanged, avoiding triggers
        AND party_member.perms != perms.perms;
    END IF;

    RETURN NEW;
END
$$;

CREATE TRIGGER role_update AFTER INSERT OR UPDATE OR DELETE ON lantern.roles
FOR EACH ROW EXECUTE FUNCTION lantern.on_role_update_trigger();

CREATE OR REPLACE FUNCTION lantern.on_overwrite_update_trigger()
RETURNS trigger
LANGUAGE plpgsql AS
$$
DECLARE
    _role_id bigint;
    _user_id bigint;
    _room_id bigint;
BEGIN
    -- skip unchanged/spurious
    IF NEW.allow = OLD.allow AND NEW.deny = OLD.deny THEN
        RETURN NEW;
    END IF;

    SELECT
        COALESCE(NEW.role_id, OLD.role_id),
        COALESCE(NEW.user_id, OLD.user_id),
        COALESCE(NEW.room_id, OLD.room_id)
    INTO _role_id, _user_id, _room_id;

    IF _role_id IS NOT NULL THEN
        WITH members_to_update AS (
            -- will return 0 rows on @everyone, because @everyone doesn't use role_members at all
            SELECT role_members.user_id FROM lantern.role_members WHERE role_members.role_id = _role_id
            UNION ALL
            -- will only return rows when roles.id = roles.party_id, indicating @everyone
            SELECT party_member.user_id
            FROM lantern.roles INNER JOIN lantern.party_members ON party_member.party_id = roles.party_id
            WHERE roles.id = _role_id
            AND   roles.id = roles.party_id
        ), perms AS (
            SELECT
                m.user_id,
                -- user_allow | (allow & !user_deny), NULL if 0
                NULLIF(COALESCE(bit_or(o.user_allow), 0) | (COALESCE(bit_or(o.allow), 0) & ~COALESCE(bit_or(o.user_deny), 0)), 0) AS allow,
                -- deny | user_deny, NULL if 0
                NULLIF(COALESCE(bit_or(o.deny), 0) | COALESCE(bit_or(o.user_deny)), 0) AS deny
            FROM lantern.agg_overwrites o
            INNER JOIN members_to_update m ON o.user_id = m.user_id -- limit to users with this role
            WHERE o.room_id = _room_id -- limit by room, of course
            GROUP BY m.user_id
        )
        UPDATE lantern.room_members
            SET allow = perms.allow, deny = perms.deny
            FROM perms
            WHERE room_members.user_id = perms.user_id
            AND   room_members.room_id = _room_id
            AND room_members.allow IS DISTINCT FROM perms.allow
            AND room_members.deny  IS DISTINCT FROM perms.deny;

    ELSIF _user_id IS NOT NULL THEN
        WITH perms AS (
            SELECT
                -- user_allow | (allow & !user_deny), NULL if 0
                NULLIF(COALESCE(bit_or(o.user_allow), 0) | (COALESCE(bit_or(o.allow), 0) & ~COALESCE(bit_or(o.user_deny), 0)), 0) AS allow,
                -- deny | user_deny, NULL if 0
                NULLIF(COALESCE(bit_or(o.deny), 0) | COALESCE(bit_or(o.user_deny)), 0) AS deny
            FROM lantern.agg_overwrites o
            WHERE o.user_id = _user_id
              AND o.room_id = _room_id
        )
        UPDATE lantern.room_members
            SET allow = perms.allow, deny = perms.deny
            FROM perms
            WHERE room_members.user_id = _user_id
            AND   room_members.room_id = _room_id
            AND room_members.allow IS DISTINCT FROM perms.allow
            AND room_members.deny  IS DISTINCT FROM perms.deny;
    END IF;

    RETURN NEW;
END
$$;

CREATE TRIGGER overwrite_update AFTER UPDATE OR INSERT OR DELETE ON lantern.overwrites
FOR EACH ROW EXECUTE FUNCTION lantern.on_overwrite_update_trigger();

CREATE OR REPLACE FUNCTION lantern.on_party_member_exist()
RETURNS trigger
LANGUAGE plpgsql AS
$$
BEGIN
    IF TG_OP = 'DELETE' THEN
        DELETE FROM lantern.room_members m
        USING lantern.rooms
        WHERE m.user_id = OLD.user_id
        AND m.room_id = rooms.id
        AND rooms.party_id = OLD.party_id;
    ELSIF TG_OP = 'INSERT' THEN
        INSERT INTO lantern.room_members (user_id, room_id, allow, deny)
        SELECT NEW.user_id, o.room_id,
        -- user_allow | (allow & !user_deny), NULL if 0
        NULLIF(COALESCE(bit_or(o.user_allow), 0) | (COALESCE(bit_or(o.allow), 0) & ~COALESCE(bit_or(o.user_deny), 0)), 0) AS allow,
        -- deny | user_deny, NULL if 0
        NULLIF(COALESCE(bit_or(o.deny), 0) | COALESCE(bit_or(o.user_deny)), 0) AS deny
        FROM lantern.agg_overwrites o INNER JOIN lantern.rooms ON rooms.id = o.room_id
        WHERE rooms.party_id = NEW.party_id;
    END IF;

    RETURN NEW;
END
$$;

CREATE TRIGGER room_member_update AFTER INSERT OR DELETE ON lantern.party_member
FOR EACH ROW EXECUTE FUNCTION lantern.on_party_member_exist();

-- When a new room is created, the room_members must have values inserted
CREATE OR REPLACE FUNCTION lantern.on_room_add()
RETURNS trigger
LANGUAGE plpgsql AS
$$
BEGIN
    INSERT INTO lantern.room_members (user_id, room_id, allow, deny)
    SELECT party_member.user_id, NEW.room_id, NULL, NULL
    FROM lantern.party_member WHERE party_member.party_id = NEW.party_id;

    RETURN NEW;
END
$$;

CREATE TRIGGER room_insert_member AFTER INSERT ON lantern.rooms
FOR EACH ROW EXECUTE FUNCTION lantern.on_room_add();

-- when users are given or removed from a role, update their permissions
CREATE OR REPLACE FUNCTION lantern.on_role_member_modify()
RETURNS trigger
LANGUAGE plpgsql AS
$$
DECLARE
    _user_id bigint;
    _role_id bigint;
BEGIN
    SELECT
        COALESCE(NEW.user_id, OLD.user_id),
        COALESCE(NEW.role_id, OLD.role_id)
    INTO _user_id, _role_id;

    -- update per-room cached permissions first
    WITH r AS (
        -- get all rooms in party based on the role given/removed
        SELECT rooms.id AS room_id, rooms.party_id
        FROM lantern.rooms INNER JOIN lantern.roles ON roles.party_id = rooms.party_id
        WHERE roles.id = _role_id
    ), perms AS (
        -- iterate through rooms and accumulate overwrites
        SELECT
            o.room_id,
            -- user_allow | (allow & !user_deny), NULL if 0
            NULLIF(COALESCE(bit_or(o.user_allow), 0) | (COALESCE(bit_or(o.allow), 0) & ~COALESCE(bit_or(o.user_deny), 0)), 0) AS allow,
            -- deny | user_deny, NULL if 0
            NULLIF(COALESCE(bit_or(o.deny), 0) | COALESCE(bit_or(o.user_deny)), 0) AS deny
        FROM lantern.agg_overwrites o
        INNER JOIN r ON o.room_id = r.room_id AND o.user_id = _user_id
        GROUP BY o.room_id
    )
    UPDATE lantern.room_members
        SET allow = perms.allow, deny = perms.deny
        FROM perms
        WHERE room_members.user_id = _user_id
        AND room_members.room_id = perms.room_id
        AND room_members.allow IS DISTINCT FROM perms.allow
        AND room_members.deny IS DISTINCT FROM perms.deny;

    -- update per-party permissions
    WITH p AS (
        -- get party_id of role being given/removed
        SELECT roles.party_id
        FROM lantern.roles
        WHERE roles.id = _role_id
    ), perms AS (
        -- pass through p.party_id to limit party_member below
        SELECT p.party_id, bit_or(roles.permissions) AS perms
        FROM lantern.role_members
        INNER JOIN lantern.roles ON roles.id = role_members.role_id
        INNER JOIN p ON roles.party_id = p.party_id
        GROUP BY p.party_id
    )
    UPDATE lantern.party_member SET perms = perms.perms
    FROM perms WHERE party_member.user_id = _user_id
    AND party_member.party_id = perms.party_id
    AND party_member.perms != perms.perms;

    RETURN NEW;
END
$$;

CREATE TRIGGER role_member_modify AFTER INSERT OR DELETE ON lantern.role_members
FOR EACH ROW EXECUTE FUNCTION lantern.on_role_member_modify();

-----------------------------------------
---------------- VIEWS ------------------
-----------------------------------------


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

---

CREATE OR REPLACE VIEW lantern.agg_original_profile_files(
    user_id,
    party_id,
    bits,
    avatar_file_id,
    banner_file_id
) AS
SELECT
    profiles.user_id,
    profiles.party_id,
    profiles.bits,
    avatar_asset.file_id,
    banner_asset.file_id
FROM
    lantern.profiles
    LEFT JOIN lantern.user_assets avatar_asset ON avatar_asset.id = profiles.avatar_id
    LEFT JOIN lantern.user_assets banner_asset ON banner_asset.id = profiles.banner_id
;

---

CREATE OR REPLACE VIEW lantern.agg_mentions AS
SELECT mentions.msg_id,
       array_agg(CASE WHEN mentions.user_id IS NOT NULL THEN 1
                      WHEN mentions.role_id IS NOT NULL THEN 2
                      WHEN mentions.room_id IS NOT NULL THEN 3
                 END) AS kinds,
       array_agg(COALESCE(mentions.user_id, mentions.role_id, mentions.room_id)) AS ids
FROM lantern.mentions GROUP BY msg_id;

---

CREATE OR REPLACE VIEW lantern.agg_friends(user_id, friend_id, updated_at, flags, note) AS
SELECT user_a_id, user_b_id, updated_at, flags, note_a FROM lantern.friends
UNION ALL
-- xor the ADDED_BY flag if the order is reversed
SELECT user_b_id, user_a_id, updated_at, flags # 2::int2, note_b FROM lantern.friends;

--

CREATE OR REPLACE VIEW lantern.agg_overwrites(
    room_id,
    user_id,
    role_id,
    user_allow,
    user_deny,
    allow,
    deny
) AS

-- simple per-user overwrites
SELECT
    overwrites.room_id,
    overwrites.user_id,
    overwrites.role_id,
    overwrites.allow,
    overwrites.deny, 0, 0

FROM lantern.overwrites WHERE user_id IS NOT NULL

UNION ALL

-- per-role overwrites where the user has that role, automatically filtered by using role_members.user_id
SELECT
    overwrites.room_id,
    role_members.user_id,
    overwrites.role_id,
    0, 0,
    overwrites.allow,
    overwrites.deny

FROM lantern.overwrites INNER JOIN lantern.role_members ON overwrites.role_id = role_members.role_id

UNION ALL

-- at-everyone role overrides, which are roles with the same id as the party
SELECT
    overwrites.room_id,
    party_member.user_id,
    overwrites.role_id,
    0, 0, overwrites.allow, overwrites.deny

FROM
    lantern.party_member INNER JOIN
        lantern.roles INNER JOIN
            lantern.overwrites INNER JOIN lantern.rooms ON rooms.id = overwrites.room_id
        ON roles.id = rooms.party_id AND roles.id = overwrites.role_id
    ON party_member.party_id = rooms.party_id;

--

CREATE OR REPLACE VIEW lantern.agg_room_perms(room_id, user_id, perms) AS
SELECT
    rooms.id AS room_id,
    party_member.user_id,
--    roles.permissions, deny, allow, user_deny, user_allow
--    bit_or(roles.permissions), COALESCE(bit_or(deny), 0), COALESCE(bit_or(allow), 0), COALESCE(bit_or(user_deny), 0), COALESCE(bit_or(user_allow), 0)
    (((bit_or(roles.permissions) & ~COALESCE(bit_or(deny), 0)) | COALESCE(bit_or(allow), 0)) & ~COALESCE(bit_or(user_deny), 0)) | COALESCE(bit_or(user_allow), 0) |
       bit_or(CASE WHEN party.owner_id = party_member.user_id THEN -1 ELSE 0 END) AS perms

FROM
    lantern.agg_overwrites RIGHT JOIN
        lantern.roles RIGHT JOIN
            lantern.rooms INNER JOIN
                lantern.party INNER JOIN lantern.party_member ON party_member.party_id = party.id
            ON rooms.party_id = party.id
        ON roles.id = party.id
    ON agg_overwrites.room_id = rooms.id AND agg_overwrites.user_id = party_member.user_id
GROUP BY party_member.user_id, rooms.id;


--

CREATE OR REPLACE VIEW lantern.agg_attachments(
    msg_id,
    meta,
    preview
) AS
-- query this first with ORDER BY to ensure attachment order
WITH f AS (
    SELECT files.id, files.size, files.flags, files.name, files.mime, files.width, files.height, files.preview
    FROM lantern.files
    ORDER BY files.id
)
SELECT
    message_id as msg_id,
    jsonb_agg(jsonb_build_object(
        'id', files.id,
        'size', files.size,
        'flags', files.flags,
        'name', files.name,
        'mime', files.mime,
        'width', files.width,
        'height', files.height
    )) AS meta,
    array_agg(files.preview) AS preview
FROM
    lantern.attachments INNER JOIN f files ON files.id = attachments.file_id
GROUP BY
    msg_id
;

--

CREATE OR REPLACE VIEW lantern.agg_presence(
    user_id,
    flags,
    updated_at,
    activity
) AS
SELECT DISTINCT ON (user_id)
    user_id,
    flags,
    updated_at,
    activity
   FROM lantern.user_presence
  ORDER BY user_id, flags DESC, updated_at DESC
;
COMMENT ON VIEW lantern.agg_presence IS 'Returns the single most recent/priority presence';

--

CREATE OR REPLACE VIEW lantern.agg_users(
    id,
    discriminator,
    email,
    flags,
    username,
    preferences,
    presence_flags,
    presence_updated_at,
    presence_activity
)
AS
SELECT
    users.id,
    users.discriminator,
    users.email,
    users.flags,
    users.username,
    users.preferences,
    agg_presence.flags,
    agg_presence.updated_at,
    agg_presence.activity

FROM
    lantern.users LEFT JOIN lantern.agg_presence ON agg_presence.user_id = users.id
;

--

CREATE OR REPLACE VIEW lantern.agg_members(
    user_id,
    party_id,
    flags,
    joined_at,
    role_ids
) AS
SELECT
    party_member.user_id,
    party_member.party_id,
    party_member.flags,
    party_member.joined_at,
    (
        SELECT
            ARRAY_AGG(role_id) as roles
        FROM
            lantern.role_members INNER JOIN lantern.roles
            ON  roles.id = role_members.role_id
            AND roles.party_id = party_member.party_id
            AND role_members.user_id = party_member.user_id
    )
FROM
    lantern.party_member
;

--

CREATE OR REPLACE VIEW lantern.agg_members_full(
    party_id,
    user_id,
    discriminator,
    user_flags,
    username,
    presence_flags,
    presence_updated_at,
    nickname,
    member_flags,
    joined_at,
    avatar_id,
    profile_bits,
    custom_status,
    role_ids,
    presence_activity
) AS
SELECT
    party_member.party_id,
    party_member.user_id,
    agg_users.discriminator,
    agg_users.flags,
    agg_users.username,
    agg_users.presence_flags,
    agg_users.presence_updated_at,
    COALESCE(party_profile.nickname, base_profile.nickname),
    party_member.flags,
    party_member.joined_at,
    COALESCE(party_profile.avatar_id, base_profile.avatar_id),
    lantern.combine_profile_bits(base_profile.bits, party_profile.bits, party_profile.avatar_id),
    COALESCE(party_profile.custom_status, base_profile.custom_status),
    agg_roles.roles,
    agg_users.presence_activity
FROM
    lantern.party_member INNER JOIN lantern.agg_users ON (agg_users.id = party_member.user_id)
    LEFT JOIN lantern.profiles base_profile ON (base_profile.user_id = party_member.user_id AND base_profile.party_id IS NULL)
    LEFT JOIN lantern.profiles party_profile ON (party_profile.user_id = party_member.user_id AND party_profile.party_id = party_member.party_id)

    LEFT JOIN LATERAL (
        SELECT
            ARRAY_AGG(role_id) as roles
        FROM
            lantern.role_members INNER JOIN lantern.roles
            ON  roles.id = role_members.role_id
            AND roles.party_id = party_member.party_id
            AND role_members.user_id = party_member.user_id
    ) agg_roles ON TRUE
;

--

CREATE OR REPLACE VIEW lantern.agg_broadcast_visibility(user_id, other_id, party_id) AS
SELECT user_id, friend_id, NULL FROM lantern.agg_friends
UNION ALL
SELECT p.user_id, NULL, p.party_id FROM lantern.party_member p
-- UNION ALL
-- Select users from DMs that are subscribed (open) by the other members
;

--

CREATE OR REPLACE VIEW lantern.agg_user_associations(user_id, other_id, party_id) AS
SELECT user_id, friend_id, NULL FROM lantern.agg_friends f WHERE f.flags & 1 = 1
UNION ALL
SELECT my.user_id, o.user_id, my.party_id FROM
    lantern.party_member my INNER JOIN lantern.party_member o ON (o.party_id = my.party_id)
;

--

-- NOTE: Just search for `REFERENCES lantern.files` to find which tables should be here
CREATE OR REPLACE VIEW lantern.agg_used_files(id) AS
SELECT file_id FROM lantern.user_assets
UNION ALL
SELECT file_id FROM lantern.user_asset_files
UNION ALL
SELECT file_id FROM lantern.attachments
UNION ALL
SELECT wallpaper_id FROM lantern.room_members WHERE wallpaper_id IS NOT NULL
;

--

-- provided solely for the ORDER BY clause
CREATE OR REPLACE VIEW lantern.agg_reactions(
    msg_id,
    emote_id,
    reacted,
    emoji_id,
    user_ids
) AS
SELECT
    msg_id,
    emote_id,
    reacted,
    emoji_id,
    user_ids
FROM lantern.reactions
ORDER BY reacted ASC;

--

CREATE OR REPLACE VIEW lantern.agg_member_presence(
    user_id,
    discriminator,
    username,
    user_flags,

    party_id,

    profile_bits,
    nickname,
    avatar_id,
    banner_id,
    custom_status,
    biography,

    updated_at,
    presence_flags,
    presence_activity
) AS
SELECT
    users.id,
    users.discriminator,
    users.username,
    users.flags,

    party_member.party_id,

    lantern.combine_profile_bits(base_profile.bits, party_profile.bits, party_profile.avatar_id),
    COALESCE(party_profile.nickname, base_profile.nickname),
    COALESCE(party_profile.avatar_id, base_profile.avatar_id),
    COALESCE(party_profile.banner_id, base_profile.banner_id),
    COALESCE(party_profile.custom_status, base_profile.custom_status),
    COALESCE(party_profile.biography, base_profile.biography),

    presence.updated_at,
    presence.flags,
    presence.activity
FROM
    users INNER JOIN party_member ON party_member.user_id = users.id

    LEFT JOIN lantern.agg_presence presence ON presence.user_id = users.id

    LEFT JOIN lantern.profiles base_profile ON (base_profile.user_id = users.id AND base_profile.party_id IS NULL)
    LEFT JOIN lantern.profiles party_profile ON (party_profile.user_id = users.id AND party_profile.party_id = party_member.party_id)
;

-----------------------------------------
-------- PROCEDURES & FUNCTIONS ---------
-----------------------------------------

CREATE OR REPLACE PROCEDURE lantern.redeem_invite(
    _user_id bigint,
    INOUT _invite_id bigint,
    _invite_code text
)
LANGUAGE plpgsql AS
$$
DECLARE
    _party_id bigint;
    _banned bigint;
BEGIN
    UPDATE lantern.invite
        SET uses = uses - 1
    FROM
        lantern.party
            LEFT JOIN lantern.party_bans ON party_bans.party_id = party.id AND party_bans.user_id = _user_id
    WHERE
        invite.uses > 0
        AND invite.expires > now()
        AND (invite.id = _invite_id OR invite.vanity = _invite_code)
        AND party.id = invite.party_id -- ensure correct party/party_bans is selected
    RETURNING
        invite.id, invite.party_id, party_bans.user_id INTO _invite_id, _party_id, _banned;

    -- exceptions will rollback transaction
    IF _banned IS NOT NULL THEN
        RAISE EXCEPTION 'user_banned';
    ELSIF _party_id IS NULL THEN
        RAISE EXCEPTION 'invalid_invite';
    ELSE
        -- new users just start out with @everyone permissions
        WITH p AS (
            SELECT r.permissions AS perms FROM lantern.roles r WHERE r.id = _party_id
        )
        -- insert new one at the top
        -- NOTE: Using -1 and doing this insert first avoids extra rollback work on failure
        INSERT INTO lantern.party_member(party_id, user_id, invite_id, joined_at, position, perms)
        SELECT _party_id, _user_id, _invite_id, now(), -1, p.perms FROM p;

        -- move all parties down to normalize
        UPDATE lantern.party_member
            SET position = position + 1
        WHERE
            party_member.user_id = _user_id;
    END IF;
END
$$;

--

CREATE OR REPLACE PROCEDURE lantern.register_user(
   _id bigint,
   _username text,
   _email text,
   _passhash text,
   _dob date
)
LANGUAGE plpgsql AS
$$
DECLARE
   _discriminator lantern.uint2;
BEGIN
    SELECT discriminator INTO _discriminator FROM lantern.user_freelist WHERE username = _username;

    IF FOUND THEN
        DELETE FROM lantern.user_freelist WHERE username = _username AND discriminator = _discriminator;
    ELSE
        SELECT discriminator INTO _discriminator FROM lantern.users WHERE username = _username ORDER BY discriminator DESC LIMIT 1;

        IF NOT FOUND THEN
            _discriminator := 0;
        ELSIF _discriminator = 65535 THEN
            RAISE EXCEPTION 'Username % exhausted', _username;
        ELSE
            _discriminator := _discriminator + 1;
        END IF;
    END IF;

    INSERT INTO lantern.users (id, username, discriminator, email, passhash, dob) VALUES (_id, _username, _discriminator, _email, _passhash, _dob);
END
$$;

--

CREATE OR REPLACE PROCEDURE lantern.update_user(
    _id bigint,
    _username text,
    _email text,
    _passhash text
)
LANGUAGE plpgsql AS
$$
DECLARE
    _discriminator lantern.uint2;
BEGIN
    IF _username IS NOT NULL THEN
        SELECT discriminator INTO _discriminator FROM lantern.user_freelist WHERE username = _username;

        IF FOUND THEN
            DELETE FROM lantern.user_freelist WHERE username = _username AND discriminator = _discriminator;
        ELSE
            SELECT discriminator INTO _discriminator FROM lantern.users WHERE username = _username ORDER BY discriminator DESC LIMIT 1;

            IF NOT FOUND THEN
                _discriminator := 0;
            ELSIF _discriminator = 65535 THEN
                RAISE EXCEPTION 'Username % exhausted', _username;
            ELSE
                _discriminator := _discriminator + 1;
            END IF;
        END IF;

        -- Add current user's username to the freelist once found
        INSERT INTO lantern.user_freelist (SELECT username, discriminator FROM lantern.users WHERE users.id = _id);
    END IF;

    UPDATE lantern.users SET
        username        = COALESCE(_username,       username),
        discriminator   = COALESCE(_discriminator,  discriminator),
        email           = COALESCE(_email,          email),
        passhash        = COALESCE(_passhash,       passhash)
    WHERE
        users.id = _id;
END
$$;

--

CREATE OR REPLACE PROCEDURE lantern.upsert_msg(
    _id bigint,
    _user_id bigint,
    _room_id bigint,
    _thread_id bigint,
    _editor_id bigint,
    _updated_at timestamptz,
    _deleted_at timestamptz,
    _content text,
    _pinned bool
)
LANGUAGE plpgsql AS
$$
BEGIN
    INSERT INTO lantern.messages (id, user_id, room_id, thread_id, editor_id, updated_at, deleted_at, content, pinned)
    VALUES (_id, _user_id, _room_id, _thread_id, _editor_id, _updated_at, _deleted_at, _content, _pinned)
    ON CONFLICT ON CONSTRAINT messages_pk DO
        UPDATE SET user_id = _user_id, room_id = _room_id, thread_id = _thread_id,
                   editor_id = _editor_id, updated_at = _updated_at, deleted_at = _deleted_at,
                   pinned = _pinned;
END
$$;

--

CREATE OR REPLACE PROCEDURE lantern.set_user_status(
    _user_id bigint,
    _active smallint
)
LANGUAGE plpgsql AS
$$
DECLARE
    _now timestamptz := now();
BEGIN
    INSERT INTO lantern.user_status (id, updated, active) VALUES (_user_id, _now, _active)
    ON CONFLICT ON CONSTRAINT user_status_pk DO
        UPDATE SET updated = _now, active = _active;
END
$$;

--

CREATE OR REPLACE PROCEDURE lantern.set_presence(
    _user_id bigint,
    _conn_id bigint,
    _flags   smallint,
    _activity jsonb
)
LANGUAGE plpgsql AS
$$
BEGIN
    INSERT INTO lantern.user_presence (user_id, conn_id, updated_at, flags, activity)
    VALUES (_user_id, _conn_id, now(), _flags, _activity)
    ON CONFLICT ON CONSTRAINT presence_pk DO
        UPDATE SET updated_at   = now(),
                   flags        = _flags,
                   activity     = _activity;
END
$$;

--

CREATE OR REPLACE FUNCTION lantern.create_thread(
    _thread_id bigint,
    _parent_id bigint,
    _new_flags smallint
)
RETURNS bigint
LANGUAGE plpgsql AS
$$
DECLARE
    _existing_thread_id bigint;
BEGIN
    -- fast case, replying to a thread that's already been started
    SELECT threads.id INTO _existing_thread_id FROM lantern.threads WHERE threads.parent_id = _parent_id;
    IF FOUND THEN
        RETURN _existing_thread_id;
    END IF;

    -- edge case, replying to a reply to a thread, use the ancestor thread_id
    SELECT thread_id INTO _existing_thread_id FROM lantern.messages WHERE messages.id = _parent_id;
    IF _existing_thread_id IS NOT NULL THEN
        RETURN _existing_thread_id;
    END IF;

    -- normal case, create a new thread
    INSERT INTO lantern.threads (id, parent_id, flags) VALUES (_thread_id, _parent_id, _new_flags);

    RETURN _thread_id;
END
$$;

--

CREATE OR REPLACE PROCEDURE lantern.soft_delete_user(
    _user_id bigint,
    _new_username text
)
LANGUAGE plpgsql AS
$$
BEGIN
    UPDATE lantern.users SET deleted_at = now() WHERE id = _user_id;
    CALL lantern.update_user(_user_id, _new_username);
    DELETE FROM lantern.sessions WHERE user_id = _user_id;
    DELETE FROM lantern.user_tokens WHERE user_id = _user_id;
    DELETE FROM lantern.user_presence WHERE user_id = _user_id;
    DELETE FROM lantern.profiles WHERE user_id = _user_id;
    DELETE FROM lantern.friends WHERE user_a_id = _user_id OR user_b_id = _user_id;
    DELETE FROM lantern.user_blocks WHERE user_id = _user_id OR block_id = _user_id;
    DELETE FROM lantern.party_bans WHERE user_id = _user_id;
    DELETE FROM lantern.overrides WHERE user_id = _user_id;
    DELETE FROM lantern.role_members WHERE user_id = _user_id;
    DELETE FROM lantern.party_member WHERE user_id = _user_id;
END
$$;