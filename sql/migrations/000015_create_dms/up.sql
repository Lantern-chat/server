CREATE TABLE lantern.dms (
    user_id_a   bigint      NOT NULL,
    user_id_b   bigint      NOT NULL,
    room_id     bigint      NOT NULL,
    CONSTRAINT dm_pk PRIMARY KEY (user_id_a, user_id_b)
);
ALTER TABLE lantern.dms OWNER TO postgres;

CREATE INDEX CONCURRENTLY dm_user_a_idx ON lantern.dms USING hash(user_id_a);
CREATE INDEX CONCURRENTLY dm_user_b_idx ON lantern.dms USING hash(user_id_b);

ALTER TABLE lantern.dms ADD CONSTRAINT user_id_a_fk FOREIGN KEY (user_id_a)
    REFERENCES lantern.users (id) MATCH FULL
    ON DELETE NO ACTION ON UPDATE CASCADE; -- Leave DM open on user deletion?

ALTER TABLE lantern.dms ADD CONSTRAINT user_id_b_fk FOREIGN KEY (user_id_b)
    REFERENCES lantern.users (id) MATCH FULL
    ON DELETE NO ACTION ON UPDATE CASCADE; -- Leave DM open on user deletion?

ALTER TABLE lantern.dms ADD CONSTRAINT room_id_fk FOREIGN KEY (room_id)
    REFERENCES lantern.rooms (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE; -- delete DM if channel is deleted?

CREATE TABLE lantern.groups (
    id          bigint      NOT NULL,
    room_id     bigint      NOT NULL,

    CONSTRAINT group_pk PRIMARY KEY (id)
);
ALTER TABLE lantern.groups OWNER TO postgres;

CREATE TABLE lantern.group_members (
    group_id    bigint      NOT NULL,
    user_id     bigint      NOT NULL,

    CONSTRAINT group_member_pk PRIMARY KEY (group_id, user_id)
);
ALTER TABLE lantern.group_members OWNER TO postgres;

CREATE INDEX CONCURRENTLY group_member_id_idx ON lantern.group_members USING hash(group_id);
CREATE INDEX CONCURRENTLY group_member_user_idx ON lantern.group_members USING hash(user_id);

ALTER TABLE lantern.group_members ADD CONSTRAINT group_id_fk FOREIGN KEY (group_id)
    REFERENCES lantern.groups (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE; -- Delete members if whole group is deleted

ALTER TABLE lantern.group_members ADD CONSTRAINT user_id_fk FOREIGN KEY (user_id)
    REFERENCES lantern.users (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE; -- Delete member if user is deleted