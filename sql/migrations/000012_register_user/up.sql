CREATE OR REPLACE PROCEDURE lantern.register_user(
   _id bigint,
   _username varchar(64),
   _email text,
   _passhash text,
   _dob date
)
LANGUAGE plpgsql AS
$$
DECLARE
   _discriminator lantern.uint2;
BEGIN
    SELECT discriminator INTO _discriminator FROM lantern.user_freelist WHERE username = _username;

    IF FOUND THEN
        DELETE FROM lantern.user_freelist WHERE username = _username AND discriminator = _discriminator;
    ELSE
        SELECT discriminator INTO _discriminator FROM lantern.users WHERE username = _username ORDER BY discriminator DESC LIMIT 1;

        IF NOT FOUND THEN
            _discriminator := 0;
        ELSIF _discriminator = 65535 THEN
            RAISE EXCEPTION 'Username % exhausted', _username;
        ELSE
            _discriminator := _discriminator + 1;
        END IF;
    END IF;

    INSERT INTO lantern.users (id, username, discriminator, email, passhash, dob) VALUES (_id, _username, _discriminator, _email, _passhash, _dob);
END
$$;

CREATE OR REPLACE PROCEDURE lantern.update_user(
    _id bigint,
    _username varchar(64),
    _email text,
    _passhash text
)
LANGUAGE plpgsql AS
$$
DECLARE
    _discriminator lantern.uint2;
BEGIN
    IF _username IS NOT NULL THEN
        SELECT discriminator INTO _discriminator FROM lantern.user_freelist WHERE username = _username;

        IF FOUND THEN
            DELETE FROM lantern.user_freelist WHERE username = _username AND discriminator = _discriminator;
        ELSE
            SELECT discriminator INTO _discriminator FROM lantern.users WHERE username = _username ORDER BY discriminator DESC LIMIT 1;

            IF NOT FOUND THEN
                _discriminator := 0;
            ELSIF _discriminator = 65535 THEN
                RAISE EXCEPTION 'Username % exhausted', _username;
            ELSE
                _discriminator := _discriminator + 1;
            END IF;
        END IF;

        -- Add current user's username to the freelist once found
        INSERT INTO lantern.user_freelist (SELECT username, discriminator FROM lantern.users WHERE users.id = _id);
    END IF;

    UPDATE lantern.users SET
        username        = COALESCE(_username,       username),
        discriminator   = COALESCE(_discriminator,  discriminator),
        email           = COALESCE(_email,          email),
        passhash        = COALESCE(_passhash,       passhash)
    WHERE
        users.id = _id;
END
$$;