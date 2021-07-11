DROP PROCEDURE IF EXISTS lantern.set_presence(bigint, bigint, smallint, jsonb) CASCADE;

DROP TRIGGER IF EXISTS presence_update ON lantern.user_presence CASCADE;

DROP FUNCTION IF EXISTS lantern.presence_trigger() CASCADE;

DROP INDEX IF EXISTS user_presence_idx CASCADE;
DROP INDEX IF EXISTS user_presence_conn_idx CASCADE;

DROP TABLE IF EXISTS lantern.user_presence CASCADE;