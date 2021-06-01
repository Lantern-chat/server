-- Users can have multiple avatars, with one main avatar
CREATE TABLE lantern.user_avatars (
    id          bigint  NOT NULL,
    user_id     bigint  NOT NULL,
    file_id     bigint  NOT NULL,
    is_main     bool    NOT NULL DEFAULT false,

    CONSTRAINT user_avatars_pk PRIMARY KEY(id)
);
ALTER TABLE lantern.user_avatars OWNER TO postgres;

CREATE UNIQUE INDEX user_avatars_main_idx ON lantern.user_avatars
    USING btree(user_id, is_main) WHERE is_main IS NOT FALSE;

CREATE INDEX user_avatars_user_idx ON lantern.user_avatars USING hash(user_id);

ALTER TABLE lantern.user_avatars ADD CONSTRAINT user_fk FOREIGN KEY(user_id)
    REFERENCES lantern.users (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;

ALTER TABLE lantern.user_avatars ADD CONSTRAINT file_fk FOREIGN KEY(file_id)
    REFERENCES lantern.files (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;

--
-- Update existing tables with avatars fks
--

-- Add avatar to party
ALTER TABLE lantern.party ADD CONSTRAINT avatar_fk FOREIGN KEY (avatar_id)
    REFERENCES lantern.files (id) MATCH FULL
    ON DELETE SET NULL ON UPDATE CASCADE; -- IMPORTANT: Only set NULL when deleting avatars, DO NOT CASCADE

-- Add avatar to rooms
ALTER TABLE lantern.rooms ADD CONSTRAINT avatar_fk FOREIGN KEY (avatar_id)
    REFERENCES lantern.files (id) MATCH FULL
    ON DELETE SET NULL ON UPDATE CASCADE; -- IMPORTANT: Only set NULL when deleting avatars, DO NOT CASCADE

-- Add avatar to party member
ALTER TABLE lantern.party_member ADD CONSTRAINT avatar_fk FOREIGN KEY (avatar_id)
    REFERENCES lantern.user_avatars (id) MATCH FULL
    ON DELETE SET NULL ON UPDATE CASCADE; -- IMPORTANT: Only set NULL when deleting avatars, DO NOT CASCADE