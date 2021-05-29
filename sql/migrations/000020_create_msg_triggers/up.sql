CREATE OR REPLACE FUNCTION lantern.msg_trigger()
RETURNS trigger
LANGUAGE plpgsql AS
$$
DECLARE
    _code smallint;
    _party_id bigint;
    _id bigint;
BEGIN
    IF OLD IS NOT NULL AND
       OLD.deleted_at IS NOT NULL AND
       OLD.deleted_at != NEW.deleted_at
    THEN
        _code := 3;
    ELSE
        _code := CASE WHEN TG_OP = 'INSERT' THEN 1
                      WHEN TG_OP = 'UPDATE' THEN 2
                 END;
    END IF;

    _party_id := (SELECT party_id FROM lantern.rooms WHERE rooms.id = NEW.room_id);
    _id := NEW.id;

    INSERT INTO lantern.event_log (code, id, party_id) VALUES(_code, _id, _party_id);

    RETURN NEW;
END
$$;

CREATE TRIGGER message_event AFTER UPDATE OR INSERT ON lantern.messages
FOR EACH ROW EXECUTE FUNCTION lantern.msg_trigger();