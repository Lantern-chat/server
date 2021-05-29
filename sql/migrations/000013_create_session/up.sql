CREATE TABLE lantern.sessions (
    user_id bigint      NOT NULL,
    expires timestamp   NOT NULL,
    addr    inet        NOT NULL,
    token   bytea       NOT NULL
);
ALTER TABLE lantern.sessions OWNER TO postgres;

CREATE INDEX CONCURRENTLY session_token_idx ON lantern.sessions
    USING hash (token);

CREATE INDEX CONCURRENTLY session_expires_idx ON lantern.sessions
    USING btree (expires);

ALTER TABLE lantern.sessions ADD CONSTRAINT user_fk FOREIGN KEY (user_id)
    REFERENCES lantern.users (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;