ALTER TABLE lantern.thread_subscriptions DROP CONSTRAINT IF EXISTS thread_fk CASCADE;
ALTER TABLE lantern.thread_subscriptions DROP CONSTRAINT IF EXISTS user_fk CASCADE;

DROP TABLE IF EXISTS lantern.thread_subscriptions;

ALTER TABLE lantern.threads DROP CONSTRAINT IF EXISTS message_fk CASCADE;
ALTER TABLE lantern.messages DROP CONSTRAINT IF EXISTS thread_fk CASCADE;

ALTER TABLE lantern.message DROP COLUMN IF EXISTS thread_id CASCADE;

ALTER TABLE lantern.threads DROP CONSTRAINT IF EXISTS parent_uq CASCADE;

DROP TABLE IF EXISTS lantern.threads;