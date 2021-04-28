ALTER TABLE IF EXISTS lantern.group_members DROP CONSTRAINT IF EXISTS user_id_fk CASCADE;
ALTER TABLE IF EXISTS lantern.group_members DROP CONSTRAINT IF EXISTS group_id_fk CASCADE:

DROP INDEX IF EXISTS lantern.group_member_user_idx CASCADE;
DROP INDEX IF EXISTS lantern.group_member_id_idx CASCADE;

DROP TABLE IF EXISTS lantern.group_members CASCADE;

DROP TABLE IF EXISTS lantern.groups CASCADE;

ALTER TABLE IF EXISTS lantern.dms DROP CONSTRAINT IF EXISTS room_id_fk CASCADE;
ALTER TABLE IF EXISTS lantern.dms DROP CONSTRAINT IF EXISTS user_id_b_fk CASCADE;
ALTER TABLE IF EXISTS lantern.dms DROP CONSTRAINT IF EXISTS user_id_a_fk CASCADE;

DROP INDEX IF EXISTS lantern.dm_user_b_idx CASCADE;
DROP INDEX IF EXISTS lantern.dm_user_a_idx CASCADE;

DROP TABLE IF EXISTS lantern.dms CASCADE;