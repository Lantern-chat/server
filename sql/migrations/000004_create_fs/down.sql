DROP PROCEDURE IF EXISTS lantern.upsert_file(bigint, text, bytea, text, int, int, smallint, bytea);

DROP INDEX IF EXISTS file_hash_idx CASCADE;

DROP TABLE IF EXISTS lantern.files CASCADE;