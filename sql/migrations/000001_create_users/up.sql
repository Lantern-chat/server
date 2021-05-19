CREATE TABLE lantern.users (
    --- Snowflake id
    id              bigint              NOT NULL,
    deleted_at      timestamp,
    username        varchar(64)         NOT NULL,
    -- 2-byte integer that can be displayed as 4 hex digits
    discriminator   smallint            NOT NULL,
    email           text                NOT NULL,
    dob             date                NOT NULL,
    flags           smallint            NOT NULL    DEFAULT false,
    passhash        text                NOT NULL,
    -- custom_status tracks the little blurb that appears on users
    custom_status   varchar(128),
    -- biography is an extended user description on their profile
    biography       varchar(1024),
    -- this is for client-side user preferences, which can be stored as JSON easily enough
    preferences     jsonb,

    -- 0/NULL for online, 1 for away, 2 for busy, 3 for invisible
    away            smallint,

    CONSTRAINT users_pk PRIMARY KEY (id)
);
ALTER TABLE lantern.users OWNER TO postgres;

-- Fast lookup of users with identical usernames
CREATE INDEX CONCURRENTLY user_username_idx ON lantern.users
    USING hash (username);

-- Fast lookup of users via `username#0000`
CREATE INDEX CONCURRENTLY user_username_discriminator_idx ON lantern.users
    USING btree (username, discriminator);

CREATE TABLE lantern.users_freelist (
    username        varchar(64) NOT NULL,
    descriminator   smallint    NOT NULL
);
ALTER TABLE lantern.users_freelist OWNER TO postgres;

CREATE INDEX CONCURRENTLY user_freelist_username_idx ON lantern.users_freelist
    USING hash (username);