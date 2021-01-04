ALTER TABLE lantern.attachments DROP CONSTRAINT IF EXISTS file_fk CASCADE;
ALTER TABLE lantern.attachments DROP CONSTRAINT IF EXISTS attachment_eq CASCADE;
ALTER TABLE lantern.attachments DROP CONSTRAINT IF EXISTS message_fk CASCADE;

DROP INDEX IF EXISTS lantern.msg_room_idx CASCADE;
DROP INDEX IF EXISTS lantern.msg_user_idx CASCADE;
DROP INDEX IF EXISTS lantern.msg_id_idx CASCADE;

DROP TABLE IF EXISTS lantern.attachments;

ALTER TABLE lantern.messages DROP CONSTRAINT IF EXISTS editor_fk CASCADE;
ALTER TABLE lantern.messages DROP CONSTRAINT IF EXISTS user_fk CASCADE;
ALTER TABLE lantern.messages DROP CONSTRAINT IF EXISTS room_fk CASCADE;

DROP TABLE IF EXISTS lantern.messages;