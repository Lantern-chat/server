CREATE TABLE lantern.avatar (
    id      bigint NOT NULL,
    file_id bigint NOT NULL,

    CONSTRAINT avatar_pk PRIMARY KEY (id)
);
ALTER TABLE lantern.avatar OWNER TO postgres;

-- Avatar references one file from files
ALTER TABLE lantern.avatar ADD CONSTRAINT avatar_uq UNIQUE (file_id);
ALTER TABLE lantern.avatar ADD CONSTRAINT file_fk FOREIGN KEY (file_id)
    REFERENCES lantern.files (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE; -- If there is no file, then just delete the whole avatar


--
-- Update existing tables with avatars fks
--

-- Add avatar to users
ALTER TABLE lantern.users ADD CONSTRAINT avatar_fk FOREIGN KEY (avatar_id)
    REFERENCES lantern.avatar (id) MATCH FULL
    ON DELETE SET NULL ON UPDATE CASCADE; -- IMPORTANT: Only set NULL when deleting avatars, DO NOT CASCADE

-- Add avatar to party
ALTER TABLE lantern.party ADD CONSTRAINT avatar_fk FOREIGN KEY (avatar_id)
    REFERENCES lantern.avatar (id) MATCH FULL
    ON DELETE SET NULL ON UPDATE CASCADE; -- IMPORTANT: Only set NULL when deleting avatars, DO NOT CASCADE

-- Add avatar to rooms
ALTER TABLE lantern.rooms ADD CONSTRAINT avatar_fk FOREIGN KEY (avatar_id)
    REFERENCES lantern.avatar (id) MATCH FULL
    ON DELETE SET NULL ON UPDATE CASCADE; -- IMPORTANT: Only set NULL when deleting icons, DO NOT CASCADE
