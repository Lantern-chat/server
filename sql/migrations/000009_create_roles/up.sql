CREATE TABLE lantern.roles (
    id              bigint      NOT NULL,
    party_id        bigint      NOT NULL,
    avatar_id       bigint,
    -- Actually contains 3 16-bit fields
    permissions     bigint      NOT NULL    DEFAULT 0,
    color           integer,
    position        smallint    NOT NULL    DEFAULT 0,
    flags           smallint    NOT NULL    DEFAULT 0,
    name            varchar(32),

    CONSTRAINT role_pk PRIMARY KEY (id)
);
ALTER TABLE lantern.roles OWNER TO postgres;

ALTER TABLE lantern.roles ADD CONSTRAINT unique_role_position UNIQUE(party_id, position) DEFERRABLE INITIALLY DEFERRED;

-- faster lookup `WHERE party_id`
CREATE INDEX role_party_idx ON lantern.roles USING hash (party_id);

ALTER TABLE lantern.roles ADD CONSTRAINT party_fk FOREIGN KEY (party_id)
    REFERENCES lantern.party (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;

ALTER TABLE lantern.roles ADD CONSTRAINT avatar_fk FOREIGN KEY (avatar_id)
    REFERENCES lantern.files (id) MATCH FULL
    ON DELETE SET NULL ON UPDATE CASCADE;

-- Role/User association map
-- The party id can be found by joining with the role itself
CREATE TABLE lantern.role_members (
    role_id    bigint NOT NULL,
    user_id    bigint NOT NULL,

    CONSTRAINT role_member_pk PRIMARY KEY (role_id, user_id)
);
ALTER TABLE lantern.role_members OWNER TO postgres;

ALTER TABLE lantern.role_members ADD CONSTRAINT role_fk FOREIGN KEY (role_id)
    REFERENCES lantern.roles (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;

ALTER TABLE lantern.role_members ADD CONSTRAINT user_fk FOREIGN KEY (user_id)
    REFERENCES lantern.users (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;