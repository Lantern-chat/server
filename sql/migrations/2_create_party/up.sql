--
-- Party table
--

CREATE TABLE lantern.party (
    id          bigint          NOT NULL,
    name        varchar(256)    NOT NULL,
    -- If NULL, it's a private chat/DM
    owner_id    bigint,

    -- Inactive parties are banned/deleted until purged
    is_active   bool            NOT NULL DEFAULT true,

    CONSTRAINT party_pk PRIMARY KEY (id)
);
ALTER TABLE lantern.party OWNER TO postgres;

CREATE UNIQUE INDEX CONCURRENTLY name_idx ON lantern.party USING btree (name);

ALTER TABLE lantern.party ADD CONSTRAINT owner_fk FOREIGN KEY (owner_id)
    REFERENCES lantern.users (id) MATCH FULL
    ON DELETE RESTRICT ON UPDATE CASCADE; -- Don't allow users to delete accounts if they own parties




-- Association map between parties and users
CREATE TABLE lantern.party_member (
    party_id    bigint NOT NULL,
    user_id     bigint NOT NULL,

    -- Composite primary key
    CONSTRAINT party_member_pk PRIMARY KEY (party_id, user_id)
);
ALTER TABLE lantern.party_member OWNER TO postgres;

ALTER TABLE lantern.party_member ADD CONSTRAINT party_fk FOREIGN KEY (party_id)
    REFERENCES lantern.party (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE; -- When a party is deleted cascade to delete memberships

ALTER TABLE lantern.party_member ADD CONSTRAINT member_fk FOREIGN KEY (user_id)
    REFERENCES lantern.users (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE; -- When a user is deleted cascade to delete their membership
