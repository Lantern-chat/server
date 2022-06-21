-- Users can have multiple avatars, with one main avatar
CREATE TABLE lantern.user_avatars (
    user_id     bigint  NOT NULL,
    file_id     bigint  NOT NULL,
    party_id    bigint
);
ALTER TABLE lantern.user_avatars OWNER TO postgres;

CREATE UNIQUE INDEX user_avatars_user_party_idx ON lantern.user_avatars
    USING btree(user_id, COALESCE(party_id, 1));

CREATE INDEX user_avatar_file_idx ON lantern.user_avatars USING hash(file_id);

ALTER TABLE lantern.user_avatars ADD CONSTRAINT user_fk FOREIGN KEY(user_id)
    REFERENCES lantern.users (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;

ALTER TABLE lantern.user_avatars ADD CONSTRAINT party_fk FOREIGN KEY(party_id)
    REFERENCES lantern.party (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;

ALTER TABLE lantern.user_avatars ADD CONSTRAINT file_fk FOREIGN KEY(file_id)
    REFERENCES lantern.files (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE; -- On file delete, delete avatar entry (will then show default avatar)

-- ALTER TABLE user_avatars ADD CONSTRAINT user_party_uq
-- UNIQUE USING INDEX user_avatars_user_party_idx;

CREATE OR REPLACE PROCEDURE lantern.upsert_user_avatar(
    _user_id bigint,
    _party_id bigint,
    _file_id bigint
)
LANGUAGE plpgsql AS
$$
BEGIN
    INSERT INTO lantern.user_avatars (user_id, party_id, file_id)
    VALUES (_user_id, _party_id, _file_id)
    -- Ensure this conflict matches the user_avatars_user_party_idx expression
    ON CONFLICT (user_id, COALESCE(party_id, 1)) DO
        UPDATE SET file_id = _file_id;
END
$$;

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
