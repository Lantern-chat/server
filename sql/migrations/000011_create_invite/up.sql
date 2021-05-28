CREATE TABLE lantern.invite (
    id          bigint      NOT NULL,
    party_id    bigint      NOT NULL,
    user_id     bigint      NOT NULL,
    expires     timestamp   NOT NULL,
    uses        smallint    NOT NULL    DEFAULT 1,
    code        varchar(16) NOT NULL,
    description text        NOT NULL,

    CONSTRAINT invite_pk PRIMARY KEY (id)
);
ALTER TABLE lantern.invite OWNER TO postgres;

ALTER TABLE lantern.invite ADD CONSTRAINT party_fk FOREIGN KEY (party_id)
    REFERENCES lantern.party (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;

ALTER TABLE lantern.invite ADD CONSTRAINT user_fk FOREIGN KEY (user_id)
    REFERENCES lantern.users (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;



-- Track what invite was used to invite a member
-- ALTER TABLE lantern.party_member ADD COLUMN invite_id bigint;

ALTER TABLE lantern.party_member ADD CONSTRAINT invite_fk FOREIGN KEY (invite_id)
    REFERENCES lantern.invite (id) MATCH FULL
    ON DELETE SET NULL ON UPDATE CASCADE;