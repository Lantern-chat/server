DROP FUNCTION IF EXISTS lantern.create_thread(bigint, bigint, smallint) CASCADE;

ALTER TABLE IF EXISTS lantern.threads DROP CONSTRAINT IF EXISTS message_fk CASCADE;
ALTER TABLE IF EXISTS lantern.messages DROP CONSTRAINT IF EXISTS thread_fk CASCADE;
ALTER TABLE IF EXISTS lantern.threads DROP CONSTRAINT IF EXISTS parent_uq CASCADE;

DROP TABLE IF EXISTS lantern.threads;