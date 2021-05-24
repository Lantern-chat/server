CREATE TABLE lantern.event_log (
    code    smallint    NOT NULL,
    id      bigint      NOT NULL
);
ALTER TABLE lantern.event_log OWNER TO postgres;

CREATE INDEX event_log_idx ON lantern.event_log USING btree(id);