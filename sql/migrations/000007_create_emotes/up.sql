CREATE TABLE lantern.emotes (
    id              bigint          NOT NULL,
    party_id        bigint,
    file_id         bigint          NOT NULL,
    aspect_ratio    real            NOT NULL,
    flags           smallint        NOT NULL,
    name            varchar(64)     NOT NULL,
    alt             varchar(64),

    CONSTRAINT emotes_pk PRIMARY KEY (id)
);
ALTER TABLE lantern.emotes OWNER TO postgres;

CREATE INDEX emote_party_idx ON lantern.emotes
    USING hash(party_id);

CREATE UNIQUE INDEX emote_name_idx ON lantern.emotes
    USING btree (party_id, name);

ALTER TABLE lantern.emotes ADD CONSTRAINT party_fk FOREIGN KEY (party_id)
    REFERENCES lantern.party (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE; -- Delete emotes on party deletion

ALTER TABLE lantern.emotes ADD CONSTRAINT file_fk FOREIGN KEY (file_id)
    REFERENCES lantern.files (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;
