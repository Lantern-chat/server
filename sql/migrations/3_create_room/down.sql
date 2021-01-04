ALTER TABLE lantern.subscriptions DROP CONSTRAINT IF EXISTS user_fk CASCADE;
ALTER TABLE lantern.subscriptions DROP CONSTRAINT IF EXISTS room_fk CASCADE;

DROP TABLE IF EXISTS lantern.subscriptions;

ALTER TABLE lantern.rooms DROP CONSTRAINT IF EXISTS party_fk CASCADE;

DROP INDEX IF EXISTS lantern.room_name_idx CASCADE;

DROP TABLE IF EXISTS lantern.rooms CASCADE;