CREATE OR REPLACE PROCEDURE lantern.upsert_msg(
    _id bigint,
    _user_id bigint,
    _room_id bigint,
    _thread_id bigint,
    _editor_id bigint,
    _updated_at timestamp,
    _deleted_at timestamp,
    _content text,
    _pinned bool
)
LANGUAGE plpgsql AS
$$
BEGIN
    INSERT INTO lantern.messages (id, user_id, room_id, thread_id, editor_id, updated_at, deleted_at, content, pinned)
    VALUES (_id, _user_id, _room_id, _thread_id, _editor_id, _updated_at, _deleted_at, _content, _pinned)
    ON CONFLICT ON CONSTRAINT messages_pk DO
        UPDATE SET user_id = _user_id, room_id = _room_id, thread_id = _thread_id,
                   editor_id = _editor_id, updated_at = _updated_at, deleted_at = _deleted_at,
                   pinned = _pinned;
END
$$;