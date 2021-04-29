CREATE TABLE lantern.overrides (
    room_id         bigint      NOT NULL,

    role_id         bigint,
    user_id         bigint,

    allow           bigint      NOT NULL    DEFAULT 0,
    deny            bigint      NOT NULL    DEFAULT 0
);
ALTER TABLE lantern.overrides OWNER TO postgres;

CREATE INDEX CONCURRENTLY overrides_room_role_idx ON lantern.overrides
    USING btree(room_id, role_id) WHERE role_id IS NOT NULL;
CREATE INDEX CONCURRENTLY overrides_room_user_idx ON lantern.overrides
    USING btree(room_id, user_id) WHERE user_id IS NOT NULL;

ALTER TABLE lantern.overrides ADD CONSTRAINT room_id_fk FOREIGN KEY (room_id)
    REFERENCES lantern.rooms (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;

ALTER TABLE lantern.overrides ADD CONSTRAINT role_id_fk FOREIGN KEY (role_id)
    REFERENCES lantern.roles (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;

ALTER TABLE lantern.overrides ADD CONSTRAINT user_id_fk FOREIGN KEY (user_id)
    REFERENCES lantern.users (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;