CREATE TABLE lantern.report (
    -- Snowflake ID
    id          bigint      NOT NULL,
    message_id  bigint      NOT NULL,
    reporter_id bigint      NOT NULL,


    -- User ID of moderation staff
    resolver    bigint,
    resolved_at timestamp,

    priority    smallint    NOT NULL DEFAULT 9999,

    reason      text,

    -- If NULL, then not resolved. if not NULL, then the action taken
    resolved    text,

    CONSTRAINT report_pk PRIMARY KEY (id)
);
ALTER TABLE lantern.report OWNER TO postgres;

ALTER TABLE lantern.report ADD CONSTRAINT message_fk FOREIGN KEY (message_id)
    REFERENCES lantern.messages (id) MATCH FULL
    ON DELETE NO ACTION ON UPDATE CASCADE;

ALTER TABLE lantern.report ADD CONSTRAINT report_uq UNIQUE (message_id);

ALTER TABLE lantern.report ADD CONSTRAINT user_fk FOREIGN KEY (reporter_id)
    REFERENCES lantern.users (id) MATCH FULL
    ON DELETE NO ACTION ON UPDATE CASCADE;

ALTER TABLE lantern.report ADD CONSTRAINT resolver_fk FOREIGN KEY (resolver)
    REFERENCES lantern.users (id) MATCH FULL
    ON DELETE NO ACTION ON UPDATE CASCADE;