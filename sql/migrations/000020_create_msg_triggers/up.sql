CREATE OR REPLACE FUNCTION lantern.msg_trigger()
RETURNS trigger
LANGUAGE plpgsql AS
$$
BEGIN
    INSERT INTO lantern.event_log (code, id, party_id)
    SELECT
        -- when old was not deleted, and new is deleted
        CASE WHEN ((OLD.flags & (1 << 5)) = 0) AND ((NEW.flags & (1 << 5)) != 0)
                THEN 'message_delete'::lantern.event_code

             WHEN TG_OP = 'INSERT'
                THEN 'message_create'::lantern.event_code

             WHEN TG_OP = 'UPDATE' AND ((NEW.flags & (1 << 5)) = 0)
                THEN 'message_update'::lantern.event_code
        END,
        NEW.id,
        (SELECT party_id FROM lantern.rooms WHERE rooms.id = NEW.room_id);

    RETURN NEW;
END
$$;


DROP TRIGGER IF EXISTS message_event on lantern.messages CASCADE;
CREATE TRIGGER message_event AFTER UPDATE OR INSERT ON lantern.messages
FOR EACH ROW EXECUTE FUNCTION lantern.msg_trigger();