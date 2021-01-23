ALTER TABLE IF EXISTS lantern.sessions DROP CONSTRAINT IF EXISTS user_fk CASCADE;

DROP INDEX IF EXISTS lantern.session_expires_idx CASCADE;
DROP INDEX IF EXISTS lantern.session_id_idx CASCADE;

DROP TABLE IF EXISTS lantern.sessions;