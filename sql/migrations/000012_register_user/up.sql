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
   _discriminator smallint;
BEGIN
    SELECT descriminator INTO _discriminator FROM lantern.users_freelist WHERE username = _username;

    IF FOUND THEN
        DELETE FROM lantern.users_freelist WHERE username = _username AND discriminator = _discriminator;
    ELSE
        SELECT discriminator INTO _discriminator FROM lantern.users WHERE username = _username ORDER BY id DESC;

        IF NOT FOUND THEN
            _discriminator := 0;
        ELSIF _discriminator = -1 THEN
            RAISE EXCEPTION 'Username % exhausted', _username;
        ELSE
            _discriminator := _discriminator + 1;
        END IF;
    END IF;

    INSERT INTO lantern.users (id, username, discriminator, email, passhash, dob) VALUES (_id, _username, _discriminator, _email, _passhash, _dob);
END
$$;