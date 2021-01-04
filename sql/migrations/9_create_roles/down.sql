ALTER TABLE lantern.role_members DROP CONSTRAINT IF EXISTS user_fk CASCADE;
ALTER TABLE lantern.role_members DROP CONSTRAINT IF EXISTS role_fk CASCADE;

DROP TABLE IF EXISTS lantern.role_members;

ALTER TABLE lantern.roles DROP CONSTRAINT IF EXISTS party_fk CASCADE;

DROP INDEX IF EXISTS lantern.role_party_idx CASCADE;

DROP TABLE IF EXISTS lantern.roles CASCADE;