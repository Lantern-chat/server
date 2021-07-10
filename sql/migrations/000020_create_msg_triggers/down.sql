DROP TRIGGER IF EXISTS message_event on lantern.messages CASCADE;

DROP FUNCTION IF EXISTS lantern.msg_trigger() CASCADE;