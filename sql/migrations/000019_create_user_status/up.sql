CREATE TABLE lantern.user_status (
    user_id         bigint      NOT NULL,
    updated         timestamp   NOT NULL DEFAULT now(),
    active          smallint    NOT NULL DEFAULT 0,

    CONSTRAINT user_status_pk PRIMARY KEY (user_id)
);
ALTER TABLE lantern.user_status OWNER TO postgres;

ALTER TABLE lantern.user_status ADD CONSTRAINT user_fk FOREIGN KEY(user_id)
    REFERENCES lantern.users (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;

CREATE INDEX user_status_user_idx ON lantern.user_status USING hash(user_id);
CREATE INDEX user_status_time_idx ON lantern.user_status USING btree(updated);

CREATE OR REPLACE PROCEDURE lantern.set_user_status(
    _user_id bigint,
    _active smallint
)
LANGUAGE plpgsql AS
$$
DECLARE
    _now timestamp := now();
BEGIN
    INSERT INTO lantern.user_status (id, updated, active) VALUES (_user_id, _now, _active)
    ON CONFLICT ON CONSTRAINT user_status_pk DO
        UPDATE SET updated = _now, active = _active;
END
$$;