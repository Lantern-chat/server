CREATE TABLE lantern.user_presence (
    user_id     bigint      NOT NULL,
    -- Connection ID, only really seen on the server layer
    conn_id     bigint      NOT NULL,
    updated_at  timestamp   NOT NULL DEFAULT now(),
    flags       smallint    NOT NULL,
    activity    jsonb
);
ALTER TABLE lantern.user_presence OWNER TO postgres;

CREATE INDEX user_presence_conn_idx ON lantern.user_presence USING hash(conn_id);
CREATE INDEX user_presence_idx ON lantern.user_presence USING btree(user_id, updated_at);

ALTER TABLE lantern.user_presence ADD CONSTRAINT user_fk FOREIGN KEY(user_id)
    REFERENCES lantern.users (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;

CREATE OR REPLACE FUNCTION lantern.presence_trigger()
RETURNS trigger
LANGUAGE plpgsql AS
$$
BEGIN
    INSERT INTO lantern.event_log (code, id) VALUES (
        'presence_updated'::lantern.event_code,
        NEW.user_id
    );

    RETURN NEW;
END
$$;

CREATE TRIGGER presence_update AFTER UPDATE OR INSERT ON lantern.user_presence
FOR EACH ROW EXECUTE FUNCTION lantern.presence_trigger();