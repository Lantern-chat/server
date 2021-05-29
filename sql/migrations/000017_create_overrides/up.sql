CREATE TABLE lantern.overwrites (
    room_id         bigint      NOT NULL,

    allow           bigint      NOT NULL    DEFAULT 0,
    deny            bigint      NOT NULL    DEFAULT 0,

    role_id         bigint,
    user_id         bigint,

    CONSTRAINT overwrites_pk PRIMARY KEY (room_id)
);
ALTER TABLE lantern.overwrites OWNER TO postgres;

CREATE INDEX CONCURRENTLY overwrites_room_role_idx ON lantern.overwrites
    USING btree(room_id, role_id) WHERE role_id IS NOT NULL;
CREATE INDEX CONCURRENTLY overwrites_room_user_idx ON lantern.overwrites
    USING btree(room_id, user_id) WHERE user_id IS NOT NULL;

ALTER TABLE lantern.overwrites ADD CONSTRAINT room_id_fk FOREIGN KEY (room_id)
    REFERENCES lantern.rooms (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;

ALTER TABLE lantern.overwrites ADD CONSTRAINT role_id_fk FOREIGN KEY (role_id)
    REFERENCES lantern.roles (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;

ALTER TABLE lantern.overwrites ADD CONSTRAINT user_id_fk FOREIGN KEY (user_id)
    REFERENCES lantern.users (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;