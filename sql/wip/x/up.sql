CREATE TABLE lantern.slow_mode (
    expires    timestamp     NOT NULL,
    room_id    bigint         NOT NULL,
    user_id    bigint         NOT NULL,

    CONSTRAINT slow_mode_pk PRIMARY KEY (room_id,user_id)
);
ALTER TABLE lantern.slow_mode OWNER TO postgres;

CREATE INDEX  CONCURRENTLY expires_idx ON lantern.slow_mode USING btree (expires);

ALTER TABLE lantern.slow_mode ADD CONSTRAINT room_fk FOREIGN KEY (room_id)
REFERENCES lantern.rooms (id) MATCH FULL
ON DELETE SET NULL ON UPDATE CASCADE;

ALTER TABLE lantern.slow_mode ADD CONSTRAINT slow_mode_uq UNIQUE (room_id);

ALTER TABLE lantern.slow_mode ADD CONSTRAINT user_fk FOREIGN KEY (user_id)
REFERENCES lantern.users (id) MATCH FULL
ON DELETE SET NULL ON UPDATE CASCADE;

ALTER TABLE lantern.slow_mode ADD CONSTRAINT slow_mode_uq1 UNIQUE (user_id);





CREATE TABLE lantern.room_role (
    room_id bigint NOT NULL,
    user_id bigint NOT NULL,
    role_id bigint NOT NULL,

    CONSTRAINT room_role_pk PRIMARY KEY (room_id,user_id,role_id)
);
ALTER TABLE lantern.room_role OWNER TO postgres;

ALTER TABLE lantern.room_role ADD CONSTRAINT room_fk FOREIGN KEY (room_id)
REFERENCES lantern.rooms (id) MATCH FULL
ON DELETE SET NULL ON UPDATE CASCADE;

ALTER TABLE lantern.room_role ADD CONSTRAINT user_fk FOREIGN KEY (user_id)
REFERENCES lantern.users (id) MATCH FULL
ON DELETE SET NULL ON UPDATE CASCADE;

ALTER TABLE lantern.room_role ADD CONSTRAINT role_fk FOREIGN KEY (role_id)
REFERENCES lantern.roles (id) MATCH FULL
ON DELETE SET NULL ON UPDATE CASCADE;
