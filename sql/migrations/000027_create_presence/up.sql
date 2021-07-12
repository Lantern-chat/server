CREATE TABLE lantern.user_presence (
    user_id     bigint      NOT NULL,
    -- Connection ID, only really seen on the server layer
    conn_id     bigint      NOT NULL,
    updated_at  timestamp   NOT NULL DEFAULT now(),
    flags       smallint    NOT NULL,
    activity    jsonb,

    CONSTRAINT presence_pk PRIMARY KEY (user_id, conn_id)
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
        CASE TG_OP WHEN 'DELETE' THEN OLD.user_id
                                 ELSE NEW.user_id
        END
    );

    RETURN NEW;
END
$$;

CREATE TRIGGER presence_update AFTER INSERT OR UPDATE OR DELETE ON lantern.user_presence
FOR EACH ROW EXECUTE FUNCTION lantern.presence_trigger();

CREATE OR REPLACE PROCEDURE lantern.set_presence(
    _user_id bigint,
    _conn_id bigint,
    _flags   smallint,
    _activity jsonb
)
LANGUAGE plpgsql AS
$$
BEGIN
    INSERT INTO lantern.user_presence (user_id, conn_id, updated_at, flags, activity)
    VALUES (_user_id, _conn_id, now(), _flags, _activity)
    ON CONFLICT ON CONSTRAINT presence_pk DO
        UPDATE SET updated_at   = now(),
                   flags        = _flags,
                   activity     = _activity;
END
$$;