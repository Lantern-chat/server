-- Clear avatar_id constraints
ALTER TABLE lantern.users DROP CONSTRAINT IF EXISTS avatar_fk CASCADE;
ALTER TABLE lantern.party DROP CONSTRAINT IF EXISTS avatar_fk CASCADE;
ALTER TABLE lantern.rooms DROP CONSTRAINT IF EXISTS avatar_fk CASCADE;

-- Drop avatar_id columns
ALTER TABLE lantern.users DROP COLUMN IF EXISTS avatar_id CASCADE;
ALTER TABLE lantern.party DROP COLUMN IF EXISTS avatar_id CASCADE;
ALTER TABLE lantern.rooms DROP COLUMN IF EXISTS avatar_id CASCADE;

-- Drop avatar file_fk
ALTER TABLE lantern.avatar DROP CONSTRAINT IF EXISTS file_fk CASCADE;
ALTER TABLE lantern.avatar DROP CONSTRAINT IF EXISTS avatar_uq CASCADE;

-- Drop avatar
DROP TABLE IF EXISTS lantern.avatar CASCADE;