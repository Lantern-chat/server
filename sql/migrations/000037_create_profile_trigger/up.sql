CREATE OR REPLACE FUNCTION lantern.profile_trigger()
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

CREATE TRIGGER profile_event AFTER UPDATE OR INSERT OR DELETE ON lantern.profiles
FOR EACH ROW EXECUTE FUNCTION lantern.profile_trigger();


-- When a party_member row is deleted, also delete their per-party profile override entry

CREATE OR REPLACE FUNCTION lantern.party_member_delete_profile_trigger()
RETURNS trigger
LANGUAGE plpgsql AS
$$
BEGIN
    DELETE FROM lantern.profiles WHERE user_id = OLD.user_id AND party_id = OLD.party_id;
END
$$;

CREATE TRIGGER party_member_delete_profile_event AFTER DELETE ON lantern.party_member
FOR EACH ROW EXECUTE FUNCTION lantern.party_member_delete_profile_trigger();