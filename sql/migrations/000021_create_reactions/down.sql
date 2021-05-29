DROP PROCEDURE IF EXISTS lantern.remove_reaction(bigint, bigint, bigint) CASCADE;
DROP PROCEDURE IF EXISTS lantern.add_reaction(bigint, bigint, bigint) CASCADE;

DROP INDEX IF EXISTS lantern.reaction_msg_idx CASCADE;

ALTER TABLE IF EXISTS lantern.reactions DROP CONSTRAINT msg_fk CASCADE;
ALTER TABLE IF EXISTS lantern.reactions DROP CONSTRAINT emote_fk CASCADE;

DROP TABLE IF EXISTS lantern.reactions CASCADE;