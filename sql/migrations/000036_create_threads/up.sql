CREATE TABLE IF NOT EXISTS lantern.threads (
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

CREATE OR REPLACE FUNCTION lantern.create_thread(
    _thread_id bigint,
    _parent_id bigint,
    _new_flags smallint
)
RETURNS bigint
LANGUAGE plpgsql AS
$$
DECLARE
    _existing_thread_id bigint;
BEGIN
    -- fast case, replying to a thread that's already been started
    SELECT threads.id INTO _existing_thread_id FROM lantern.threads WHERE threads.parent_id = _parent_id;
    IF FOUND THEN
        RETURN _existing_thread_id;
    END IF;

    -- edge case, replying to a reply to a thread, use the ancestor thread_id
    SELECT thread_id INTO _existing_thread_id FROM lantern.messages WHERE messages.id = _parent_id;
    IF _existing_thread_id IS NOT NULL THEN
        RETURN _existing_thread_id;
    END IF;

    -- normal case, create a new thread
    INSERT INTO lantern.threads (id, parent_id, flags) VALUES (_thread_id, _parent_id, _new_flags);

    RETURN _thread_id;
END
$$;