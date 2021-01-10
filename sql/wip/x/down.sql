ALTER TABLE lantern.messages DROP CONSTRAINT IF EXISTS editor_fk CASCADE;
ALTER TABLE lantern.party DROP CONSTRAINT IF EXISTS party_uq CASCADE;
ALTER TABLE lantern.subscription DROP CONSTRAINT IF EXISTS user_fk CASCADE;
ALTER TABLE lantern.subscription DROP CONSTRAINT IF EXISTS room_fk CASCADE;

DROP TABLE IF EXISTS lantern.subscription CASCADE;

ALTER TABLE lantern.room_role DROP CONSTRAINT IF EXISTS role_fk CASCADE;
ALTER TABLE lantern.room_role DROP CONSTRAINT IF EXISTS user_fk CASCADE;
ALTER TABLE lantern.room_role DROP CONSTRAINT IF EXISTS room_fk CASCADE;

DROP TABLE IF EXISTS lantern.room_role CASCADE;

ALTER TABLE lantern.role_member DROP CONSTRAINT IF EXISTS party_member_fk CASCADE;
ALTER TABLE lantern.party_member DROP CONSTRAINT IF EXISTS user_fk CASCADE;
ALTER TABLE lantern.party_member DROP CONSTRAINT IF EXISTS party_fk CASCADE;

DROP TABLE IF EXISTS lantern.party_member CASCADE;

ALTER TABLE lantern.last_read_message DROP CONSTRAINT IF EXISTS room_fk CASCADE;
ALTER TABLE lantern.thread DROP CONSTRAINT IF EXISTS parent_uq CASCADE;
ALTER TABLE lantern.thread DROP CONSTRAINT IF EXISTS message_fk CASCADE;
ALTER TABLE lantern.messages DROP CONSTRAINT IF EXISTS thread_fk CASCADE;

DROP TABLE IF EXISTS lantern.thread CASCADE;

ALTER TABLE lantern.role DROP CONSTRAINT IF EXISTS party_fk CASCADE;
ALTER TABLE lantern.emotes DROP CONSTRAINT IF EXISTS party_fk CASCADE;
ALTER TABLE lantern.invite DROP CONSTRAINT IF EXISTS party_fk CASCADE;
ALTER TABLE lantern.room DROP CONSTRAINT IF EXISTS party_fk CASCADE;

DROP INDEX IF EXISTS lantern.name_idx CASCADE;


ALTER TABLE lantern.last_read_message DROP CONSTRAINT IF EXISTS user_fk CASCADE;
ALTER TABLE lantern.last_read_message DROP CONSTRAINT IF EXISTS last_read_message_uq CASCADE;
ALTER TABLE lantern.last_read_message DROP CONSTRAINT IF EXISTS message_fk CASCADE;

DROP TABLE IF EXISTS lantern.last_read_message CASCADE;

ALTER TABLE lantern.invite DROP CONSTRAINT IF EXISTS user_fk CASCADE;

DROP TABLE IF EXISTS lantern.invite CASCADE;

ALTER TABLE lantern.slow_mode DROP CONSTRAINT IF EXISTS slow_mode_uq1 CASCADE;
ALTER TABLE lantern.slow_mode DROP CONSTRAINT IF EXISTS user_fk CASCADE;
ALTER TABLE lantern.slow_mode DROP CONSTRAINT IF EXISTS slow_mode_uq CASCADE;
ALTER TABLE lantern.slow_mode DROP CONSTRAINT IF EXISTS room_fk CASCADE;

DROP INDEX IF EXISTS lantern.expires_idx CASCADE;

DROP TABLE IF EXISTS lantern.slow_mode CASCADE;

DROP INDEX IF EXISTS lantern.pinned_idx CASCADE;

ALTER TABLE lantern.attachment DROP CONSTRAINT IF EXISTS attachment_uq CASCADE;
ALTER TABLE lantern.attachment DROP CONSTRAINT IF EXISTS file_fk CASCADE;

DROP TABLE IF EXISTS lantern.file CASCADE;

ALTER TABLE lantern.room DROP CONSTRAINT IF EXISTS room_uq CASCADE;
ALTER TABLE lantern.users DROP CONSTRAINT IF EXISTS user_uq CASCADE;

DROP INDEX IF EXISTS lantern.emote_name_idx CASCADE;
DROP TABLE IF EXISTS lantern.emotes CASCADE;

ALTER TABLE lantern.report DROP CONSTRAINT IF EXISTS report_uq1 CASCADE;
ALTER TABLE lantern.report DROP CONSTRAINT IF EXISTS user_fk CASCADE;
ALTER TABLE lantern.report DROP CONSTRAINT IF EXISTS report_uq CASCADE;
ALTER TABLE lantern.report DROP CONSTRAINT IF EXISTS message_fk CASCADE;

DROP TABLE IF EXISTS lantern.report CASCADE;

ALTER TABLE lantern.messages DROP CONSTRAINT IF EXISTS message_uq CASCADE;

DROP INDEX IF EXISTS lantern.room_name_idx CASCADE;


ALTER TABLE lantern.role_member DROP CONSTRAINT IF EXISTS role_fk CASCADE;

DROP TABLE IF EXISTS lantern.role_member CASCADE;
DROP TABLE IF EXISTS lantern.role CASCADE;

ALTER TABLE lantern.attachment DROP CONSTRAINT IF EXISTS message_fk CASCADE;

DROP TABLE IF EXISTS lantern.attachment CASCADE;

ALTER TABLE lantern.messages DROP CONSTRAINT IF EXISTS user_fk CASCADE;
ALTER TABLE lantern.messages DROP CONSTRAINT IF EXISTS room_fk CASCADE;

DROP TABLE IF EXISTS lantern.room CASCADE;

DROP TYPE IF EXISTS lantern.room_kind CASCADE;

DROP TABLE IF EXISTS lantern.messages CASCADE;