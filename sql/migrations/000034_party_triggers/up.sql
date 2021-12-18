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
    IF TG_OP = 'UPDATE' AND OLD.position != NEW.position THEN
        -- Force a self-update to refresh party positions
        INSERT INTO lantern.event_log(code, id, party_id)
        VALUES('self_updated'::lantern.event_code, OLD.user_id, OLD.party_id);

        RETURN NEW;
    END IF;

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

-- Updating role_members should trigger a member_updated event
CREATE OR REPLACE FUNCTION lantern.role_member_trigger()
RETURNS trigger
LANGUAGE plpgsql AS
$$
BEGIN
    INSERT INTO lantern.event_log (code, id, party_id)
    SELECT 'member_updated'::lantern.event_code,
        COALESCE(OLD.user_id, NEW.user_id),
        roles.party_id
    FROM lantern.roles WHERE roles.id = COALESCE(OLD.role_id, NEW.role_id);

    RETURN NEW;
END
$$;

DROP TRIGGER IF EXISTS role_member_event ON lantern.role_members CASCADE;
CREATE TRIGGER role_member_event AFTER UPDATE OR INSERT OR DELETE ON lantern.role_members
FOR EACH ROW EXECUTE FUNCTION lantern.role_member_trigger();

CREATE OR REPLACE FUNCTION lantern.role_trigger()
RETURNS trigger
LANGUAGE plpgsql AS
$$
BEGIN

    IF TG_OP = 'DELETE' THEN
        INSERT INTO lantern.event_log (code, id, party_id)
        VALUES ('role_deleted'::lantern.event_code, OLD.id, OLD.party_id);
    ELSE
        INSERT INTO lantern.event_log(code, id, party_id)
        SELECT
            CASE
                WHEN TG_OP = 'INSERT'
                    THEN 'role_created'::lantern.event_code
                    ELSE 'role_updated'::lantern.event_code
            END,
            NEW.id,
            NEW.party_id;

    END IF;

    RETURN NEW;
END
$$;

DROP TRIGGER IF EXISTS role_event ON lantern.roles CASCADE;
CREATE TRIGGER role_event AFTER UPDATE OR INSERT OR DELETE ON lantern.roles
FOR EACH ROW EXECUTE FUNCTION lantern.role_trigger();