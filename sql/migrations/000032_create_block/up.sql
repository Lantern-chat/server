CREATE TABLE IF NOT EXISTS lantern.user_blocks (
    user_id     bigint NOT NULL,
    block_id    bigint NOT NULL,

    blocked_at  timestamp NOT NULL DEFAULT now(),

    CONSTRAINT user_blocks_pk PRIMARY KEY (user_id, block_id)
);
ALTER TABLE lantern.user_blocks OWNER TO postgres;

ALTER TABLE lantern.user_blocks ADD CONSTRAINT user_fk FOREIGN KEY (user_id)
    REFERENCES lantern.users (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;

ALTER TABLE lantern.user_blocks ADD CONSTRAINT block_fk FOREIGN KEY (block_id)
    REFERENCES lantern.users (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;

CREATE INDEX user_block_user_idx ON lanterrn.user_blocks USING btree(user_id);