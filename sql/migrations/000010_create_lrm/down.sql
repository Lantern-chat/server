ALTER TABLE IF EXISTS lantern.lrm DROP CONSTRAINT IF EXISTS room_fk CASCADE;
ALTER TABLE IF EXISTS lantern.lrm DROP CONSTRAINT IF EXISTS user_fk CASCADE;
ALTER TABLE IF EXISTS lantern.lrm DROP CONSTRAINT IF EXISTS lrm_uq CASCADE;
ALTER TABLE IF EXISTS lantern.lrm DROP CONSTRAINT IF EXISTS message_fk CASCADE;

DROP TABLE IF EXISTS lantern.lrm;