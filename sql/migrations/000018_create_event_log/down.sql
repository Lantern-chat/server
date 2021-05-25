DROP TRIGGER event_log_notify ON lantern.event_log_last_notification CASCADE;
DROP FUNCTION IF EXISTS lantern.ev_notify() CASCADE;

DROP TABLE IF EXISTS lantern.event_log_last_notification CASCADE;

DROP INDEX IF EXISTS lantern.event_log_idx CASCADE;
ALTER TABLE IF EXISTS lantern.roles DROP CONSTRAINT IF EXISTS party_fk CASCADE;
DROP TABLE IF EXISTS lantern.event_log CASCADE;