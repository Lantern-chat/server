DROP TRIGGER IF EXISTS avatar_event ON lantern.user_avatars CASCADE;

DROP FUNCTION IF EXISTS lantern.avatar_trigger() CASCADE;