CREATE TABLE lantern.emotes (
    id              bigint          NOT NULL,
    party_id        bigint          NOT NULL,
    name            varchar(64)     NOT NULL,
    alt             varchar(64),
    flags           smallint        NOT NULL,
    aspect_ratio    real            NOT NULL,
    data            bytea           NOT NULL,

    CONSTRAINT emotes_pk PRIMARY KEY (id)
);
ALTER TABLE lantern.emotes OWNER TO postgres;

-- TODO: Maybe deduplicate this? Depends on how many duplicate names there are
CREATE UNIQUE INDEX CONCURRENTLY emote_name_idx ON lantern.emotes USING btree (name);

ALTER TABLE lantern.emotes ADD CONSTRAINT party_fk FOREIGN KEY (party_id)
    REFERENCES lantern.party (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE; -- Delete emotes on party deletion