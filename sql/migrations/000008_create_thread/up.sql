
CREATE TABLE lantern.threads (
    id             bigint NOT NULL,
    -- The first message that started the thread
    parent_id      bigint NOT NULL,

    CONSTRAINT thread_pk PRIMARY KEY (id)
);
ALTER TABLE lantern.threads OWNER TO postgres;

-- A message can only be the parent of a single thread
ALTER TABLE lantern.threads ADD CONSTRAINT parent_uq UNIQUE (parent_id);

-- Add `thread_id` to messages
ALTER TABLE lantern.messages ADD COLUMN thread_id bigint;

ALTER TABLE lantern.messages ADD CONSTRAINT thread_fk FOREIGN KEY (thread_id)
    REFERENCES lantern.threads (id) MATCH FULL
    ON DELETE SET NULL ON UPDATE CASCADE;

-- Don't allow parent messages to be deleted, they must be handled specially
ALTER TABLE lantern.threads ADD CONSTRAINT message_fk FOREIGN KEY (parent_id)
    REFERENCES lantern.messages (id) MATCH FULL
    ON DELETE RESTRICT ON UPDATE CASCADE;



-- Maps what users are subscribed to what threads
CREATE TABLE lantern.thread_subscriptions {
    user_id     bigint NOT NULL,
    thread_id   bigint NOT NULL,

    CONSTRAINT thread_subscriptions_pk PRIMARY KEY (user_id, thread_id)
};
ALTER TABLE lantern.thread_subscriptions OWNER TO postgres;

ALTER TABLE lantern.thread_subscriptions ADD CONSTRAINT user_fk FOREIGN KEY (user_id)
    REFERENCES lantern.users (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;

ALTER TABLE lantern.thread_subscriptions ADD CONSTRAINT thread_fk FOREIGN KEY (thread_id)
    REFERENCES lantern.threads (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;