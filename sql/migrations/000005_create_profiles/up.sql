-- Users can have multiple profiles, with one main profile where the `party_id` is NULL
CREATE TABLE lantern.profiles (
    user_id         bigint  NOT NULL,
    party_id        bigint,
    avatar_id       bigint,
    banner_id       bigint,
    bits            int NOT NULL DEFAULT 0,
    custom_status   text,
    biography       text
);
ALTER TABLE lantern.profiles OWNER TO postgres;

-- ensure there can only be one profile per user per party (or no party)
CREATE UNIQUE INDEX profiles_user_party_idx ON lantern.profiles
    USING btree(user_id, COALESCE(party_id, 1));

ALTER TABLE lantern.profiles ADD CONSTRAINT user_fk FOREIGN KEY(user_id)
    REFERENCES lantern.users (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;

ALTER TABLE lantern.profiles ADD CONSTRAINT party_fk FOREIGN KEY(party_id)
    REFERENCES lantern.party (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;

ALTER TABLE lantern.profiles ADD CONSTRAINT avatar_fk FOREIGN KEY(avatar_id)
    REFERENCES lantern.user_assets (id) MATCH FULL
    ON DELETE SET NULL ON UPDATE CASCADE;

ALTER TABLE lantern.profiles ADD CONSTRAINT banner_fk FOREIGN KEY(banner_id)
    REFERENCES lantern.user_assets (id) MATCH FULL
    ON DELETE SET NULL ON UPDATE CASCADE;

CREATE OR REPLACE VIEW lantern.agg_profiles(
    user_id,
    party_id,
    avatar_id,
    banner_id,
    bits,
    custom_status,
    biography
) AS
SELECT
    base_profile.user_id,
    party_profile.party_id,
    COALESCE(party_profile.avatar_id, base_profile.avatar_id),
    COALESCE(party_profile.banner_id, base_profile.banner_id),
    CASE
        WHEN party_profile.party_id IS NULL
            THEN COALESCE(base_profile.bits, 0)
        ELSE

        -- Select lower 7 avatar bits
        (x'7F'::int & CASE
            WHEN party_profile.avatar_id IS NOT NULL
                THEN party_profile.bits
            ELSE base_profile.bits
        END) |
        -- Select higher 25 banner bits
        (x'FFFFFF80'::int & CASE
            -- pick out 8th bit, which signifies whether to override banner color
            WHEN party_profile.bits & 128 != 0
                THEN party_profile.bits
            ELSE base_profile.bits
        END)
    END,
    COALESCE(party_profile.custom_status, base_profile.custom_status),
    COALESCE(party_profile.biography, base_profile.biography)
FROM
    lantern.profiles base_profile LEFT JOIN lantern.profiles party_profile
        ON (party_profile.user_id = base_profile.user_id AND party_profile.party_id IS NOT NULL)
    WHERE base_profile.party_id IS NULL
;

CREATE OR REPLACE VIEW lantern.agg_original_profile_files(
    user_id,
    party_id,
    bits,
    avatar_file_id,
    banner_file_id
) AS
SELECT
    profiles.user_id,
    profiles.party_id,
    profiles.bits,
    avatar_asset.file_id,
    banner_asset.file_id
FROM
    lantern.profiles
    LEFT JOIN lantern.user_assets avatar_asset ON avatar_asset.id = profiles.avatar_id
    LEFT JOIN lantern.user_assets banner_asset ON banner_asset.id = profiles.banner_id
;