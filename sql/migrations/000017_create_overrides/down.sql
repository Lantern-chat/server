ALTER TABLE IF EXISTS lantern.overrides DROP CONSTRAINT IF EXISTS user_id_fk CASCADE;
ALTER TABLE IF EXISTS lantern.overrides DROP CONSTRAINT IF EXISTS role_id_fk CASCADE;
ALTER TABLE IF EXISTS lantern.overrides DROP CONSTRAINT IF EXISTS room_id_fk CASCADE;

DROP INDEX IF EXISTS lantern.overrides_room_user_idx CASCADE;
DROP INDEX IF EXISTS lantern.overrides_room_role_idx CASCADE;

DROP TABLE IF EXISTS lantern.overrides CASCADE;