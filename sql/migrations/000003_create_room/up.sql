CREATE TABLE lantern.rooms (
    id          bigint              NOT NULL,
    party_id    bigint,
    avatar_id   bigint,
    parent_id   bigint,
    deleted_at  timestamp,
    position    smallint            NOT NULL,
    flags       smallint            NOT NULL    DEFAULT 0,
    name        varchar(128)        NOT NULL,
    topic       varchar(2048),

    CONSTRAINT room_pk PRIMARY KEY (id)
);
ALTER TABLE lantern.rooms OWNER TO postgres;

CREATE INDEX room_name_idx ON lantern.rooms USING hash (name);
CREATE INDEX room_avatar_idx ON lantern.rooms USING hash(avatar_id);

ALTER TABLE lantern.rooms ADD CONSTRAINT party_fk FOREIGN KEY (party_id)
    REFERENCES lantern.party (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE; -- Delete rooms if party is deleted

ALTER TABLE lantern.rooms ADD CONSTRAINT parent_fk FOREIGN KEY (parent_id)
    REFERENCES lantern.rooms (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE; -- Delete rooms if whole category is deleted


CREATE TABLE lantern.subscriptions (
    user_id         bigint      NOT NULL,
    room_id         bigint      NOT NULL,

    -- If NULL, there is no mute
    mute_expires    timestamp,

    flags           smallint    NOT NULL DEFAULT 0,

    CONSTRAINT subscription_pk PRIMARY KEY (room_id, user_id)
);
ALTER TABLE lantern.subscriptions OWNER TO postgres;

ALTER TABLE lantern.subscriptions ADD CONSTRAINT room_fk FOREIGN KEY (room_id)
    REFERENCES lantern.rooms (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;

ALTER TABLE lantern.subscriptions ADD CONSTRAINT user_fk FOREIGN KEY (user_id)
    REFERENCES lantern.users (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;