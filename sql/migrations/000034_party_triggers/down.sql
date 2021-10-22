DROP TRIGGER IF EXISTS member_event ON lantern.party_member CASCADE;
DROP FUNCTION IF EXISTS lantern.member_trigger() CASCADE;