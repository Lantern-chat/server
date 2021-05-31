DROP VIEW IF EXISTS lantern.agg_mentions CASCADE;

ALTER TABLE IF EXISTS lantern.mentions DROP CONSTRAINT room_fk CASCADE;
ALTER TABLE IF EXISTS lantern.mentions DROP CONSTRAINT role_fk CASCADE;
ALTER TABLE IF EXISTS lantern.mentions DROP CONSTRAINT user_fk CASCADE;
ALTER TABLE IF EXISTS lantern.mentions DROP CONSTRAINT msg_fk CASCADE;

DROP INDEX IF EXISTS lantern.mention_role_idx CASCADE;
DROP INDEX IF EXISTS lantern.mention_user_idx CASCADE;
DROP INDEX IF EXISTS lantern.mention_msg_idx CASCADE;

DROP TABLE IF EXISTS lantern.mentions CASCADE;