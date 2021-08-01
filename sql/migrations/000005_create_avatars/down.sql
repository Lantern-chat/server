-- Clear avatar_id constraints
ALTER TABLE IF EXISTS lantern.party DROP CONSTRAINT IF EXISTS avatar_fk CASCADE;
ALTER TABLE IF EXISTS lantern.rooms DROP CONSTRAINT IF EXISTS avatar_fk CASCADE;

DROP TABLE IF EXISTS lantern.user_avatars CASCADE;