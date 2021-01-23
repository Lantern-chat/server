CREATE TABLE lantern.sessions (
    id      bytea       NOT NULL,
    user_id bigint      NOT NULL,
    expires timestamp   NOT NULL
);
ALTER TABLE lantern.sessions OWNER TO postgres;

CREATE INDEX CONCURRENTLY session_id_idx ON lantern.sessions
    USING hash (id);

CREATE INDEX CONCURRENTLY session_expires_idx ON lantern.sessions
    USING btree (expires);

ALTER TABLE lantern.sessions ADD CONSTRAINT user_fk FOREIGN KEY (user_id)
    REFERENCES lantern.users (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;