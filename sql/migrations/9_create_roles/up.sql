CREATE TABLE lantern.roles (
    id              bigint      NOT NULL,
    party_id        bigint,
    name            varchar(32) NOT NULL,
    permissions     integer     NOT NULL    DEFAULT 0,
    color           integer,
    mentionable     bool        NOT NULL    DEFAULT false,
    tagged          bool        NOT NULL    DEFAULT false,

    CONSTRAINT role_pk PRIMARY KEY (id)
);
ALTER TABLE lantern.roles OWNER TO postgres;

-- faster lookup `WHERE party_id`
CREATE INDEX CONCURRENTLY role_party_idx ON lantern.roles USING hash (party_id);

ALTER TABLE lantern.roles ADD CONSTRAINT party_fk FOREIGN KEY (party_id)
    REFERENCES lantern.party (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;



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