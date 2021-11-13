CREATE TABLE lantern.friendlist (
    user_a_id   bigint      NOT NULL,
    user_b_id   bigint      NOT NULL,
    flags       smallint    NOT NULL DEFAULT 0,
    note_a      varchar(512),
    note_b      varchar(512)
);
ALTER TABLE lantern.friendlist OWNER TO postgres;

CREATE INDEX friend_a_idx ON lantern.friendlist USING btree(user_a_id);
CREATE INDEX friend_b_idx ON lantern.friendlist USING btree(user_b_id);

ALTER TABLE lantern.friendlist ADD CONSTRAINT user_a_fk FOREIGN KEY(user_a_id)
    REFERENCES lantern.users (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;

ALTER TABLE lantern.friendlist ADD CONSTRAINT user_b_fk FOREIGN KEY(user_b_id)
    REFERENCES lantern.users (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;

CREATE OR REPLACE VIEW lantern.agg_friends(user_id, friend_id, flags, note) AS
SELECT user_a_id, user_b_id, flags, note_a FROM lantern.friendlist
UNION ALL
SELECT user_b_id, user_a_id, flags, note_b FROM lantern.friendlist;