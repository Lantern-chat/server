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
    ON DELETE CASCADE ON UPDATE CASCADE;