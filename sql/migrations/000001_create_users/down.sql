DROP INDEX IF EXISTS lantern.user_tokens_expires_idx CASCADE;
DROP INDEX IF EXISTS lantern.user_tokens_token_idx CASCADE;

ALTER TABLE IF EXISTS lantern.user_tokens DROP CONSTRAINT IF EXISTS user_fk CASCADE;

DROP TABLE IF EXISTS lantern.user_tokens CASCADE;

DROP INDEX IF EXISTS lantern.user_freelist_username_idx CASCADE;
DROP TABLE IF EXISTS lantern.users_freelist CASCADE;

DROP INDEX IF EXISTS lantern.user_email_idx CASCADE;
DROP INDEX IF EXISTS lantern.user_username_discriminator_idx CASCADE;
DROP INDEX IF EXISTS lantern.user_username_idx CASCADE;

DROP TABLE IF EXISTS lantern.users CASCADE;