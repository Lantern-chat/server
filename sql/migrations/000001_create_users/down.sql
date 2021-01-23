DROP INDEX IF EXISTS lantern.user_freelist_username_idx CASCADE;

DROP TABLE IF EXISTS lantern.users_freelist CASCADE;

DROP INDEX IF EXISTS lantern.user_username_discriminator_idx CASCADE;
DROP INDEX IF EXISTS lantern.user_username_idx CASCADE;

DROP TABLE IF EXISTS lantern.users CASCADE;