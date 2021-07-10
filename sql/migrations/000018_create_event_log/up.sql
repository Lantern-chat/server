CREATE TYPE lantern.event_code AS ENUM (
    'message_create',
    'message_update',
    'message_delete',
    'typing_started',
    'user_updated',
    'self_updated',
    'presence_updated',
    'party_create',
    'party_update',
    'party_delete',
    'room_created',
    'room_updated',
    'room_deleted',
    'member_updated',
    'member_joined',
    'member_left',
    'role_created',
    'role_updated',
    'role_deleted',
    'invite_create',
    'message_react',
    'message_unreact'
);

CREATE SEQUENCE lantern.event_id;

CREATE TABLE lantern.event_log (
    counter     bigint      NOT NULL DEFAULT nextval('lantern.event_id'),

    -- the snowflake ID of whatever this event is pointing to
    id          bigint      NOT NULL CONSTRAINT id_check CHECK (id > 0),

    -- If it's a party event, place the ID here for better throughput on application layer
    party_id    bigint,
    -- May be NULL even when the event
    room_id     bigint,

    code        event_code  NOT NULL
);
ALTER TABLE lantern.event_log OWNER TO postgres;

ALTER TABLE lantern.event_log ADD CONSTRAINT party_fk FOREIGN KEY (party_id)
    REFERENCES lantern.party (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;

ALTER TABLE lantern.event_log ADD CONSTRAINT room_fk FOREIGN KEY (room_id)
    REFERENCES lantern.rooms (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;

CREATE INDEX event_log_counter_idx ON lantern.event_log USING btree(counter);

-- CREATE INDEX event_log_party_idx ON lantern.event_log USING btree(party_id) WHERE NOT NULL;



-- Notification rate-limiting table
CREATE TABLE lantern.event_log_last_notification (
    last_notif timestamp NOT NULL DEFAULT now(),
    max_interval interval NOT NULL DEFAULT INTERVAL '100 milliseconds'
);
ALTER TABLE lantern.event_log_last_notification OWNER TO postgres;

-- Default values
INSERT INTO lantern.event_log_last_notification (last_notif, max_interval)
    VALUES (now(), '100 milliseconds');

-- Trigger function for rate-limited notifications
CREATE OR REPLACE FUNCTION lantern.ev_notify()
RETURNS trigger
LANGUAGE plpgsql AS
$$
DECLARE
    _last_notif timestamp;
    _max_interval interval;
    _now timestamp := now();
BEGIN
    SELECT
        last_notif, max_interval
    INTO
        _last_notif, _max_interval
    FROM lantern.event_log_last_notification;

    IF (_now - _last_notif) >= _max_interval THEN
        PERFORM pg_notify('event_log', (NEW.id)::text);
        UPDATE lantern.event_log_last_notification SET
            last_notif = _now;
    END IF;
    RETURN NEW;
END
$$;

CREATE TRIGGER event_log_notify AFTER INSERT ON lantern.event_log
FOR EACH ROW EXECUTE FUNCTION lantern.ev_notify();