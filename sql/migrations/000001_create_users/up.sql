CREATE TABLE lantern.users (
    --- Snowflake id
    id              bigint              NOT NULL,
    avatar_id       bigint,
    deleted_at      timestamp,
    dob             date                NOT NULL,
    flags           smallint            NOT NULL    DEFAULT 0,
    -- 2-byte integer that can be displayed as 4 hex digits
    discriminator   smallint            NOT NULL,
    username        varchar(64)         NOT NULL,
    email           text                NOT NULL,
    passhash        text                NOT NULL,
    -- custom_status tracks the little blurb that appears on users
    custom_status   varchar(128),
    -- biography is an extended user description on their profile
    biography       varchar(4096),
    -- this is for client-side user preferences, which can be stored as JSON easily enough
    preferences     jsonb,

    CONSTRAINT users_pk PRIMARY KEY (id)
);
ALTER TABLE lantern.users OWNER TO postgres;

-- Fast lookup of users with identical usernames
CREATE INDEX user_username_idx ON lantern.users
    USING hash (username);

-- Fast lookup of users via `username#0000`
CREATE INDEX user_username_discriminator_idx ON lantern.users
    USING btree (username, discriminator);

CREATE UNIQUE INDEX user_email_idx ON lantern.users
    USING btree(email);


CREATE TABLE lantern.users_freelist (
    username        varchar(64) NOT NULL,
    descriminator   smallint    NOT NULL
);
ALTER TABLE lantern.users_freelist OWNER TO postgres;

CREATE INDEX CONCURRENTLY user_freelist_username_idx ON lantern.users_freelist
    USING hash (username);

-- User verification/reset tokens
CREATE TABLE lantern.user_tokens (
    id          bigint      NOT NULL,
    user_id     bigint      NOT NULL,
    expires     timestamp   NOT NULL,
    kind        smallint    NOT NULL,
    token       bytea       NOT NULL,

    CONSTRAINT user_tokens_pk PRIMARY KEY (id)
);
ALTER TABLE lantern.user_tokens OWNER TO postgres;

ALTER TABLE lantern.user_tokens ADD CONSTRAINT user_fk FOREIGN KEY (user_id)
    REFERENCES lantern.users (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;

CREATE INDEX user_tokens_token_idx ON lantern.user_tokens
    USING hash (token);

CREATE INDEX user_tokens_expires_idx ON lantern.user_tokens
    USING btree (expires);