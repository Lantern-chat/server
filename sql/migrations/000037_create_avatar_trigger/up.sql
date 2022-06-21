CREATE OR REPLACE FUNCTION lantern.avatar_trigger()
RETURNS trigger
LANGUAGE plpgsql AS
$$
BEGIN
    INSERT INTO lantern.event_log (id, party_id, code)
    VALUES (
        COALESCE(OLD.user_id, NEW.user_id),
        COALESCE(OLD.party_id, NEW.party_id),
        'user_updated'::lantern.event_code
    );

    RETURN NEW;
END
$$;

DROP TRIGGER IF EXISTS avatar_event ON lantern.user_avatars CASCADE;
CREATE TRIGGER avatar_event AFTER UPDATE OR INSERT OR DELETE ON lantern.user_avatars
FOR EACH ROW EXECUTE FUNCTION lantern.avatar_trigger();