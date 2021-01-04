CREATE TABLE lantern.rooms (
    id 			bigint 				NOT NULL,
    -- If NULL, then it's a direct-message
    party_id 	bigint,
    name 		text 				NOT NULL,
    is_private  bool                NOT NULL DEFAULT false,
    topic 		varchar(2048),
    is_nsfw     bool                NOT NULL    DEFAULT false,

    CONSTRAINT room_pk PRIMARY KEY (id)
);
ALTER TABLE lantern.rooms OWNER TO postgres;

CREATE INDEX CONCURRENTLY room_name_idx ON lantern.rooms USING hash (name);

ALTER TABLE lantern.rooms ADD CONSTRAINT party_fk FOREIGN KEY (party_id)
    REFERENCES lantern.party (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;