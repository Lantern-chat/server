DROP TABLE IF EXISTS lantern.pin_tags CASCADE;

DROP VIEW IF EXISTS lantern.agg_used_files CASCADE;

DROP INDEX IF EXISTS lantern.msg_ts_idx;
ALTER TABLE lantern.messages DROP COLUMN IF EXISTS ts;

DROP FUNCTION lantern.to_language(int2);

DROP TRIGGER IF EXISTS profile_event ON lantern.profiles CASCADE;
DROP FUNCTION IF EXISTS lantern.profile_event() CASCADE;

DROP FUNCTION IF EXISTS lantern.create_thread(bigint, bigint, smallint) CASCADE;
DROP TABLE IF EXISTS lantern.threads;

DROP TABLE IF EXISTS lantern.embed_cache CASCADE;

DROP TRIGGER IF EXISTS member_event ON lantern.party_member CASCADE;
DROP FUNCTION IF EXISTS lantern.member_trigger() CASCADE;

DROP INDEX IF EXISTS lantern.metrics_ts_idx CASCADE;

DROP TABLE IF EXISTS lantern.metrics CASCADE;

DROP TABLE IF EXISTS user_blocks CASCADE;

DROP TABLE IF EXISTS party_bans CASCADE;

DROP VIEW IF EXISTS lantern.agg_user_associations CASCADE;
DROP VIEW IF EXISTS lantern.agg_members CASCADE;
DROP VIEW IF EXISTS lantern.agg_users CASCADE;
DROP VIEW IF EXISTS lantern.agg_presence CASCADE;

DROP VIEW IF EXISTS lantern.agg_attachments;

DROP PROCEDURE IF EXISTS lantern.set_presence(bigint, bigint, smallint, jsonb) CASCADE;

DROP TRIGGER IF EXISTS presence_update ON lantern.user_presence CASCADE;

DROP FUNCTION IF EXISTS lantern.presence_trigger() CASCADE;

DROP TABLE IF EXISTS lantern.user_presence CASCADE;

DROP VIEW IF EXISTS lantern.agg_room_perms CASCADE;
DROP VIEW IF EXISTS lantern.agg_overwrites CASCADE;

DROP VIEW IF EXISTS lantern.agg_friends CASCADE;
DROP TABLE IF EXISTS lantern.friendlist CASCADE;

DROP TABLE lantern.ip_bans CASCADE;
DROP TABLE lantern.rate_limits CASCADE;

DROP VIEW IF EXISTS lantern.agg_mentions CASCADE;
DROP TABLE IF EXISTS lantern.mentions CASCADE;

DROP PROCEDURE IF EXISTS lantern.remove_reaction(bigint, bigint, bigint) CASCADE;
DROP PROCEDURE IF EXISTS lantern.add_reaction(bigint, bigint, bigint) CASCADE;

DROP INDEX IF EXISTS lantern.reaction_msg_idx CASCADE;

ALTER TABLE IF EXISTS lantern.reactions DROP CONSTRAINT msg_fk CASCADE;
ALTER TABLE IF EXISTS lantern.reactions DROP CONSTRAINT emote_fk CASCADE;

DROP TABLE IF EXISTS lantern.reactions CASCADE;

DROP TRIGGER IF EXISTS message_event on lantern.messages CASCADE;
DROP FUNCTION IF EXISTS lantern.msg_trigger() CASCADE;

DROP PROCEDURE IF EXISTS lantern.set_user_status(bigint, smallint) CASCADE;
DROP TABLE IF EXISTS lantern.user_status CASCADE;

DROP TRIGGER IF EXISTS event_log_notify ON lantern.event_log CASCADE;
DROP FUNCTION IF EXISTS lantern.ev_notify_trigger() CASCADE;

DROP TABLE IF EXISTS lantern.event_log_last_notification CASCADE;
DROP TABLE IF EXISTS lantern.event_log CASCADE;

DROP SEQUENCE IF EXISTS lantern.event_id CASCADE;
DROP TYPE IF EXISTS lantern.event_code CASCADE;

DROP TABLE IF EXISTS lantern.overwrites CASCADE;
DROP TABLE IF EXISTS lantern.room_users CASCADE;
DROP TABLE IF EXISTS lantern.group_members CASCADE;
DROP TABLE IF EXISTS lantern.groups CASCADE;
DROP TABLE IF EXISTS lantern.dms CASCADE;

DROP PROCEDURE IF EXISTS lantern.upsert_msg(
    bigint, bigint, bigint, bigint, bigint,
    timestamp, timestamp, text, bool
);

DROP TABLE IF EXISTS lantern.sessions CASCADE;

DROP PROCEDURE IF EXISTS lantern.register_user(bigint, text, text, text, date);
DROP PROCEDURE IF EXISTS lantern.update_user(bigint, text, text, text);

ALTER TABLE IF EXISTS lantern.party_member DROP CONSTRAINT IF EXISTS invite_fk CASCADE;

DROP TABLE IF EXISTS lantern.invite CASCADE;
DROP TABLE IF EXISTS lantern.role_members CASCADE;
DROP TABLE IF EXISTS lantern.roles CASCADE;
DROP TABLE IF EXISTS lantern.emotes CASCADE;
DROP TABLE IF EXISTS lantern.attachments CASCADE;
DROP TABLE IF EXISTS lantern.messages CASCADE;

DROP VIEW IF EXISTS lantern.agg_original_profile_files CASCADE;

-- Clear avatar_id constraints
ALTER TABLE IF EXISTS lantern.party DROP CONSTRAINT IF EXISTS avatar_fk CASCADE;
ALTER TABLE IF EXISTS lantern.rooms DROP CONSTRAINT IF EXISTS avatar_fk CASCADE;

DROP TABLE IF EXISTS lantern.user_avatars CASCADE;

DROP VIEW IF EXISTS lantern.agg_assets CASCADE;

DROP PROCEDURE IF EXISTS lantern.upsert_file(bigint, text, bytea, text, int, int, smallint);

DROP TABLE IF EXISTS lantern.files CASCADE;
DROP TABLE IF EXISTS lantern.subscriptions CASCADE;
DROP TABLE IF EXISTS lantern.rooms CASCADE;
DROP TABLE IF EXISTS lantern.party_member CASCADE;
DROP TABLE IF EXISTS lantern.party CASCADE;
DROP TABLE IF EXISTS lantern.user_tokens CASCADE;
DROP TABLE IF EXISTS lantern.user_freelist CASCADE;
DROP TABLE IF EXISTS lantern.users CASCADE;

DROP TABLE IF EXISTS lantern.host CASCADE;
DROP SCHEMA IF EXISTS lantern CASCADE;