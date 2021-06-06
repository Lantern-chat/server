CREATE OR REPLACE FUNCTION lantern.msg_trigger()
RETURNS trigger
LANGUAGE plpgsql AS
$$
BEGIN
    INSERT INTO lantern.event_log (code, id, party_id)
    SELECT
        CASE WHEN OLD IS NOT NULL AND OLD.flags != NEW.flags AND (NEW.flags & (1 << 5) != 0) THEN 3
             WHEN TG_OP = 'INSERT' THEN 1
             WHEN TG_OP = 'UPDATE' THEN 2
        END,
        NEW.id,
        (SELECT party_id FROM lantern.rooms WHERE rooms.id = NEW.room_id);

    RETURN NEW;
END
$$;

CREATE TRIGGER message_event AFTER UPDATE OR INSERT ON lantern.messages
FOR EACH ROW EXECUTE FUNCTION lantern.msg_trigger();