DROP PROCEDURE IF EXISTS lantern.set_user_status(bigint, smallint) CASCADE;

DROP INDEX IF EXISTS lantern.user_status_time_idx CASCADE;
DROP INDEX IF EXISTS lantern.user_status_user_idx CASCADE;

ALTER TABLE IF EXISTS lantern.user_status DROP CONSTRAINT IF EXISTS user_fk CASCADE;

DROP TABLE IF EXISTS lantern.user_status CASCADE;