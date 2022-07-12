DROP INDEX IF EXISTS lantern.msg_ts_idx;

ALTER TABLE lantern.messages DROP COLUMN IF EXISTS ts;

DROP FUNCTION lantern.to_language(int2);