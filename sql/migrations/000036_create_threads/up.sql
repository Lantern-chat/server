CREATE TABLE lantern.threads (
    id          bigint      NOT NULL,
    -- The first message that started the thread
    parent_id   bigint      NOT NULL,

    flags       smallint    NOT NULL DEFAULT 0,

    CONSTRAINT thread_pk PRIMARY KEY (id)
);
ALTER TABLE lantern.threads OWNER TO postgres;

-- Messages can only be the parent of a single thread
ALTER TABLE lantern.threads ADD CONSTRAINT parent_uq UNIQUE (parent_id);

ALTER TABLE lantern.messages ADD CONSTRAINT thread_fk FOREIGN KEY (thread_id)
    REFERENCES lantern.threads (id) MATCH FULL
    ON DELETE SET NULL ON UPDATE CASCADE;

-- Don't allow parent messages to be deleted, they must be handled specially
ALTER TABLE lantern.threads ADD CONSTRAINT message_fk FOREIGN KEY (parent_id)
    REFERENCES lantern.messages (id) MATCH FULL
    ON DELETE RESTRICT ON UPDATE CASCADE;