CREATE TABLE IF NOT EXISTS lantern.party_bans (
    party_id    bigint NOT NULL,
    user_id     bigint NOT NULL,

    banned_at   timestamp NOT NULL DEFAULT now(),
    reason      text,

    CONSTRAINT party_bans_pk PRIMARY KEY (party_id, user_id)
);
ALTER TABLE lantern.party_bans OWNER TO postgres;

ALTER TABLE lantern.party_bans ADD CONSTRAINT party_fk FOREIGN KEY (party_id)
    REFERENCES lantern.party (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;

ALTER TABLE lantern.party_bans ADD CONSTRAINT user_fk FOREIGN KEY (user_id)
    REFERENCES lantern.users (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;