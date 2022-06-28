CREATE TABLE lantern.users (
    --- Snowflake id
    id              bigint              NOT NULL,
    deleted_at      timestamp,
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
ALTER TABLE lantern.users OWNER TO postgres;

-- Fast lookup of users with identical usernames
CREATE INDEX user_username_idx ON lantern.users
    USING hash (username);

-- Fast lookup of users via `username#0000`
CREATE UNIQUE INDEX user_username_discriminator_idx ON lantern.users
    USING btree (username, discriminator);

CREATE UNIQUE INDEX user_email_idx ON lantern.users
    USING btree(email);


CREATE TABLE lantern.user_freelist (
    username        text            NOT NULL,
    discriminator   lantern.uint2   NOT NULL
);
ALTER TABLE lantern.user_freelist OWNER TO postgres;

CREATE INDEX user_freelist_username_idx ON lantern.user_freelist
    USING hash (username);

CREATE UNIQUE INDEX user_freelist_username_discriminator_idx ON lantern.user_freelist
    USING btree (username, discriminator);

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


-- Create SYSTEM user for sending system messages

INSERT INTO lantern.users (id, dob, flags, username, discriminator, email, passhash) VALUES (1, date '1970-01-01', 256, 'SYSTEM', 0, '', '') ON CONFLICT DO NOTHING;