CREATE TABLE lantern.invite (
    id          bigint      NOT NULL,
    party_id    bigint      NOT NULL,
    user_id     bigint      NOT NULL,
    expires     timestamp   NOT NULL,
    uses        int         NOT NULL    DEFAULT 0,
    max_uses    int         NOT NULL    DEFAULT 1,
    description text        NOT NULL,
    vanity      text,

    CONSTRAINT invite_pk PRIMARY KEY (id)
);
ALTER TABLE lantern.invite OWNER TO postgres;

ALTER TABLE lantern.invite ADD CONSTRAINT party_fk FOREIGN KEY (party_id)
    REFERENCES lantern.party (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;

ALTER TABLE lantern.invite ADD CONSTRAINT user_fk FOREIGN KEY (user_id)
    REFERENCES lantern.users (id) MATCH FULL
    ON DELETE CASCADE ON UPDATE CASCADE;

CREATE UNIQUE INDEX invite_vanity_idx ON lantern.invite USING btree(vanity) WHERE vanity IS NOT NULL;


-- Track what invite was used to invite a member
-- ALTER TABLE lantern.party_member ADD COLUMN invite_id bigint;

ALTER TABLE lantern.party_member ADD CONSTRAINT invite_fk FOREIGN KEY (invite_id)
    REFERENCES lantern.invite (id) MATCH FULL
    ON DELETE SET NULL ON UPDATE CASCADE;

CREATE OR REPLACE PROCEDURE lantern.redeem_invite(
    _user_id bigint,
    INOUT _invite_id bigint,
    _invite_code text
)
LANGUAGE plpgsql AS
$$
DECLARE
    _party_id bigint;
    _banned bigint;
BEGIN
    UPDATE lantern.invite
        SET uses = uses - 1
    FROM
        lantern.party
            LEFT JOIN lantern.party_bans ON party_bans.party_id = party.id AND party_bans.user_id = _user_id
    WHERE
        invite.uses > 0
        AND invite.expires > now()
        AND (invite.id = _invite_id OR invite.vanity = _invite_code)
        AND party.id = invite.party_id -- ensure correct party/party_bans is selected
    RETURNING
        invite.id, invite.party_id, party_bans.user_id INTO _invite_id, _party_id, _banned;

    -- exceptions will rollback transaction
    IF _banned IS NOT NULL THEN
        RAISE EXCEPTION 'user_banned';
    ELSIF _party_id IS NULL THEN
        RAISE EXCEPTION 'invalid_invite';
    ELSE
        -- insert new one at the top
        -- NOTE: Using -1 and doing this insert first avoids extra rollback work on failure
        INSERT INTO lantern.party_member(party_id, user_id, invite_id, joined_at, position)
        VALUES (_party_id, _user_id, _invite_id, now(), -1);

        -- move all parties down to normalize
        UPDATE lantern.party_member
            SET position = position + 1
        WHERE
            party_member.user_id = _user_id;
    END IF;
END
$$;