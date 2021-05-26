CREATE TABLE lantern.event_log (
    code        smallint    NOT NULL,
    party_id    bigint,
    id          bigint      NOT NULL
);
ALTER TABLE lantern.event_log OWNER TO postgres;

ALTER TABLE lantern.event_log ADD CONSTRAINT party_fk FOREIGN KEY (party_id)
    REFERENCES lantern.party (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;

CREATE INDEX event_log_idx ON lantern.event_log USING btree(id);




CREATE TABLE lantern.event_log_last_notification (
    last_notif timestamp NOT NULL DEFAULT now(),
    max_interval interval NOT NULL DEFAULT INTERVAL '100 milliseconds'
);

CREATE OR REPLACE FUNCTION ev_notify()
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

    IF age(_now, _last_notif) < _max_interval THEN
        CALL pg_notify('event_log', (NEW.id)::text);
        UPDATE lantern.event_log_last_notification SET
            last_notif = _now;
    END IF;
    RETURN NEW;
END
$$;

CREATE TRIGGER event_log_notify AFTER INSERT ON lantern.event_log
FOR EACH ROW
EXECUTE FUNCTION lantern.ev_notify();