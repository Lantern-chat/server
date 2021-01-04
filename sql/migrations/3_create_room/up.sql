CREATE TABLE lantern.rooms (
    id          bigint              NOT NULL,
    party_id    bigint              NOT NULL,
    name        text                NOT NULL,
    topic       varchar(2048),

    -- contains info on NSFW, channel type, etc.
    flags       smallint            NOT NULL    DEFAULT 0,

    CONSTRAINT room_pk PRIMARY KEY (id)
);
ALTER TABLE lantern.rooms OWNER TO postgres;

CREATE INDEX CONCURRENTLY room_name_idx ON lantern.rooms USING hash (name);

ALTER TABLE lantern.rooms ADD CONSTRAINT party_fk FOREIGN KEY (party_id)
    REFERENCES lantern.party (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE; -- Delete rooms if party is deleted


CREATE TABLE lantern.subscriptions (
    user_id         bigint NOT NULL,
    room_id         bigint NOT NULL,
    mentions        bool DEFAULT true,

    -- If NULL, there is no mute
    mute_expires    timestamp,

    CONSTRAINT subscription_pk PRIMARY KEY (room_id, user_id)
);
ALTER TABLE lantern.subscriptions OWNER TO postgres;

ALTER TABLE lantern.subscriptions ADD CONSTRAINT room_fk FOREIGN KEY (room_id)
    REFERENCES lantern.rooms (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;

ALTER TABLE lantern.subscriptions ADD CONSTRAINT user_fk FOREIGN KEY (user_id)
    REFERENCES lantern.users (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;