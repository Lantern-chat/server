-- Ban lifecycle

-- Start out without a ban
-- Banned, emit memebr_ban and app code also emits member_left
-- Member is no longer visible in party, cannot rejoin
-- Member unbanned, delete member row and emit member_unban

CREATE OR REPLACE FUNCTION lantern.member_trigger()
RETURNS trigger
LANGUAGE plpgsql AS
$$
BEGIN
    IF TG_OP = 'DELETE' THEN
        INSERT INTO lantern.event_log (code, id, party_id)
        SELECT
            CASE
                -- Deleting a member entry when unbanning signifies the ban has been lifted
                -- but they must rejoin manually
                WHEN ((OLD.flags & 1 = 1)) THEN 'member_unban'::lantern.event_code
                ELSE 'member_left'::lantern.event_code
            END,
            OLD.user_id,
            OLD.party_id;
    ELSE
        INSERT INTO lantern.event_log (code, id, party_id)
        SELECT
            CASE
                WHEN TG_OP = 'INSERT'
                    THEN 'member_joined'::lantern.event_code
                WHEN ((OLD.flags & 1 = 0)) AND ((NEW.flags & 1 = 1))
                    THEN 'member_ban'::lantern.event_code
                WHEN TG_OP = 'UPDATE'
                    THEN 'member_updated'::lantern.event_code
            END,
            NEW.user_id,
            NEW.party_id;
    END IF;

    RETURN NEW;
END
$$;

DROP TRIGGER IF EXISTS member_event ON lantern.party_member CASCADE;
CREATE TRIGGER member_event AFTER UPDATE OR INSERT OR DELETE ON lantern.party_member
FOR EACH ROW EXECUTE FUNCTION lantern.member_trigger();