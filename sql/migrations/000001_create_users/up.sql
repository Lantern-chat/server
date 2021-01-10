CREATE TABLE lantern.users (
    --- Snowflake id
    id              bigint              NOT NULL,
    deleted_at      timestamp,
    username        varchar(64)         NOT NULL,
    discriminator   varchar(4)          NOT NULL,
    email           text                NOT NULL,
    is_verified     bool                NOT NULL    DEFAULT false,
    -- bcrypt string with hash and salt
    bcrypt          text                NOT NULL,
    nickname        varchar(256),
    -- custom_status tracks the little blurb that appears on users
    custom_status   varchar(128),
    -- biography is an extended user description on their profile
    biography       varchar(1024),
    -- this is for client-side user preferences, which can be stored as JSON easily enough
    preferences     jsonb               NOT NULL,

    -- 0/NULL for online, 1 for away, 2 for busy, 3 for invisible
    away            smallint,

    CONSTRAINT users_pk PRIMARY KEY (id)
);
ALTER TABLE lantern.users OWNER TO postgres;

-- Fast lookup of users via `username#0000`
CREATE INDEX CONCURRENTLY user_username_discriminator_idx ON lantern.users
    USING btree (username, discriminator);
