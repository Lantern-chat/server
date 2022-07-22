--
-- Party table
--

CREATE TABLE lantern.party (
    id              bigint      NOT NULL,
    owner_id        bigint      NOT NULL,
    -- NOTE: FK is added in later migration
    default_room    bigint      NOT NULL,
    -- packed party flags
    flags           bigint      NOT NULL DEFAULT 0,
    avatar_id       bigint,
    banner_id       bigint,
    deleted_at      timestamp,
    name            text        NOT NULL,
    description     text,

    CONSTRAINT party_pk PRIMARY KEY (id)
);
ALTER TABLE lantern.party OWNER TO postgres;

CREATE INDEX party_name_idx ON lantern.party USING btree (name);

ALTER TABLE lantern.party ADD CONSTRAINT owner_fk FOREIGN KEY (owner_id)
    REFERENCES lantern.users (id) MATCH FULL
    ON DELETE RESTRICT ON UPDATE CASCADE; -- Don't allow users to delete accounts if they own parties

-- Association map between parties and users
CREATE TABLE lantern.party_member (
    party_id    bigint      NOT NULL,
    user_id     bigint      NOT NULL,
    invite_id   bigint,
    joined_at   timestamp   NOT NULL    DEFAULT now(),
    flags       smallint    NOT NULL    DEFAULT 0,
    position    smallint    NOT NULL    DEFAULT 0,

    -- same as for user, but per-party
    nickname        text,
    custom_status   text,

    -- Composite primary key
    CONSTRAINT party_member_pk PRIMARY KEY (party_id, user_id)
);
ALTER TABLE lantern.party_member OWNER TO postgres;

-- Memberships belong to the user, so it's different from rooms/roles (which belong to the party)
ALTER TABLE lantern.party_member ADD CONSTRAINT unique_party_position UNIQUE(user_id, position) DEFERRABLE INITIALLY DEFERRED;

CREATE INDEX party_member_user_idx ON lantern.party_member USING btree(user_id);

ALTER TABLE lantern.party_member ADD CONSTRAINT party_fk FOREIGN KEY (party_id)
    REFERENCES lantern.party (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE; -- When a party is deleted cascade to delete memberships

ALTER TABLE lantern.party_member ADD CONSTRAINT member_fk FOREIGN KEY (user_id)
    REFERENCES lantern.users (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE; -- When a user is deleted cascade to delete their membership