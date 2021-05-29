CREATE TABLE lantern.reactions (
    emote_id    bigint      NOT NULL,
    msg_id      bigint      NOT NULL,
    user_ids    bigint[]    NOT NULL,

    CONSTRAINT reactions_pk PRIMARY KEY (emote_id, msg_id)
);
ALTER TABLE lantern.reactions OWNER TO postgres;

ALTER TABLE lantern.reactions ADD CONSTRAINT emote_fk FOREIGN KEY (emote_id)
    REFERENCES lantern.emotes (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;

ALTER TABLE lantern.reactions ADD CONSTRAINT msg_fk FOREIGN KEY (msg_id)
    REFERENCES lantern.messages (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;

CREATE INDEX reaction_msg_idx ON lantern.reactions USING hash(msg_id);

CREATE OR REPLACE PROCEDURE lantern.add_reaction(
    _emote_id bigint,
    _msg_id bigint,
    _user_id bigint
)
LANGUAGE sql AS
$$
    INSERT INTO lantern.reactions AS r(emote_id, msg_id, user_ids)
    VALUES (_emote_id, _msg_id, ARRAY[_user_id])
    ON CONFLICT ON CONSTRAINT reactions_pk
        DO UPDATE SET user_ids = array_append(r.user_ids, _user_id);
$$;

CREATE OR REPLACE PROCEDURE lantern.remove_reaction(
    _emote_id bigint,
    _msg_id bigint,
    _user_id bigint
)
LANGUAGE sql AS
$$
    UPDATE lantern.reactions AS r
        SET user_ids = array_remove(r.user_ids, _user_id)
    WHERE
        emote_id = _emote_id AND msg_id = _msg_id;
$$;