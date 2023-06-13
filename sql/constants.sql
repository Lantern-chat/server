
----------------------------------------
---------- USER PREFS FLAGS ------------
----------------------------------------

-- Reduce movement and animations in the UI
#define USER_PREFS_REDUCE_ANIMATIONS        (1 << 0)
-- Pause animations on window unfocus
#define USER_PREFS_UNFOCUS_PAUSE            (1 << 1)
#define USER_PREFS_LIGHT_MODE               (1 << 2)
-- Allow direct messages from shared server memmbers
#define USER_PREFS_ALLOW_DMS                (1 << 3)
-- Show small lines between message groups
#define USER_PREFS_GROUP_LINES              (1 << 4)
#define USER_PREFS_HIDE_AVATARS             (1 << 5)
-- Display dark theme in an OLED-compatible mode
#define USER_PREFS_OLED_MODE                (1 << 6)
-- Mute videos/audio by default
#define USER_PREFS_MUTE_MEDIA               (1 << 7)
-- Hide images/video with unknown dimensions
#define USER_PREFS_HIDE_UNKNOWN_DIMENSIONS  (1 << 8)
#define USER_PREFS_COMPACT_VIEW             (1 << 9)
-- Prefer browser/platform emojis rather than twemoji
#define USER_PREFS_USE_PLATFORM_EMOJIS      (1 << 10)
#define USER_PREFS_ENABLE_SPELLCHECK        (1 << 11)
#define USER_PREFS_LOW_BANDWIDTH_MODE       (1 << 12)
#define USER_PREFS_FORCE_COLOR_CONSTRAST    (1 << 13)
-- Displays information like mime type and file size
#define USER_PREFS_SHOW_MEDIA_METADATA      (1 << 14)
#define USER_PREFS_DEVELOPER_MODE           (1 << 15)
#define USER_PREFS_SHOW_DATE_CHANGE         (1 << 16)
#define USER_PREFS_HIDE_LAST_ACTIVE         (1 << 17)
#define USER_PREFS_SHOW_GREY_IMAGE_BG       (1 << 18)
#define USER_PREFS_SHOW_ATTACHMENT_GRID     (1 << 19)
#define USER_PREFS_SMALLER_ATTACHMENTS      (1 << 20)
#define USER_PREFS_HIDE_ALL_EMBEDS          (1 << 21)
#define USER_PREFS_HIDE_NSFW_EMBEDS         (1 << 22)

----------------------------------------
------------ MEMBER FLAGS --------------
----------------------------------------

#define MEMBER_BANNED (1 << 0)

----------------------------------------
------------ PROFILE FLAGS -------------
----------------------------------------

#define PROFILE_AVATAR_ROUNDNESS    127 -- x'7F'::int4
#define PROFILE_OVERRIDE_COLOR      128 -- x'80'::int4
#define PROFILE_PRIMARY_COLOR       x'FFFFFF00'::int4
#define PROFILE_COLOR_FIELDS        x'FFFFFF80'::int4

----------------------------------------
------------ MESSAGE FLAGS -------------
----------------------------------------

#define MESSAGE_DELETED  (1 << 0)
#define MESSAGE_REMOVED  (1 << 1)
#define MESSAGE_PARENT   (1 << 2)
#define MESSAGE_HAS_LINK (1 << 5)

#define MESSAGE_DELETED_PARENT     (MESSAGE_DELETED | MESSAGE_PARENT)
#define MESSAGE_DELETED_OR_REMOVED (MESSAGE_DELETED | MESSAGE_REMOVED)

----------------------------------------
--------- RELATIONSHIP FLAGS -----------
----------------------------------------

#define RELATION_NONE       0
#define RELATION_FRIEND     1
#define RELATION_BLOCKED    2

----------------------------------------
----------- PRESENCE FLAGS -------------
----------------------------------------

#define PRESENCE_OFFLINE    0
#define PRESENCE_AWAY      (1 << 0)
#define PRESENCE_MOBILE    (1 << 1)
#define PRESENCE_ONLINE    (1 << 2)
#define PRESENCE_BUSY      (1 << 3)
#define PRESENCE_INVISIBLE (1 << 4)

----------------------------------------
---------- PERMISSIONS FLAGS -----------
----------------------------------------

#define PERMISSIONS1_ADMINISTRATOR        (1 << 0)
#define PERMISSIONS1_CREATE_INVITE        (1 << 1)
#define PERMISSIONS1_KICK_MEMBERS         (1 << 2)
#define PERMISSIONS1_BAN_MEMBERS          (1 << 3)
#define PERMISSIONS1_VIEW_AUDIT_LOG       (1 << 4)
#define PERMISSIONS1_VIEW_STATISTICS      (1 << 5)
#define PERMISSIONS1_MANAGE_PARTY         (1 << 6)
#define PERMISSIONS1_MANAGE_ROOMS         (1 << 7)
#define PERMISSIONS1_MANAGE_NICKNAMES     (1 << 8)
#define PERMISSIONS1_MANAGE_ROLES         (1 << 9)
#define PERMISSIONS1_MANAGE_WEBHOOKS      (1 << 10)
-- Allows members to add or remove custom emoji, stickers or sounds.
#define PERMISSIONS1_MANAGE_EXPRESSIONS   (1 << 11)
#define PERMISSIONS1_MOVE_MEMBERS         (1 << 12)
#define PERMISSIONS1_CHANGE_NICKNAME      (1 << 13)
#define PERMISSIONS1_MANAGE_PERMS         (1 << 14)

#define PERMISSIONS1_VIEW_ROOM            (1 << 30)
#define PERMISSIONS1_READ_MESSAGE_HISTORY ((1 << 31) | PERMISSIONS1_VIEW_ROOM);
#define PERMISSIONS1_SEND_MESSAGES        ((1 << 32) | PERMISSIONS1_VIEW_ROOM);
#define PERMISSIONS1_MANAGE_MESSAGES      (1 << 33)
#define PERMISSIONS1_MUTE_MEMBERS         (1 << 34)
#define PERMISSIONS1_DEAFEN_MEMBERS       (1 << 35)
#define PERMISSIONS1_MENTION_EVERYONE     (1 << 36)
#define PERMISSIONS1_USE_EXTERNAL_EMOTES  (1 << 37)
#define PERMISSIONS1_ADD_REACTIONS        (1 << 38)
#define PERMISSIONS1_EMBED_LINKS          (1 << 39)
#define PERMISSIONS1_ATTACH_FILES         (1 << 40)
#define PERMISSIONS1_USE_SLASH_COMMANDS   (1 << 41)
#define PERMISSIONS1_SEND_TTS_MESSAGES    (1 << 42)
-- Allows a user to add new attachments to
-- existing messages using the "edit" API
#define PERMISSIONS1_EDIT_NEW_ATTACHMENT  (1 << 43)

-- Allows a user to broadcast a stream to this room
#define PERMISSIONS1_STREAM               (1 << 60)
-- Allows a user to connect and watch/listen to streams in a room
#define PERMISSIONS1_CONNECT              (1 << 61)
-- Allows a user to speak in a room without broadcasting a stream
#define PERMISSIONS1_SPEAK                (1 << 62)
-- Allows a user to acquire priority speaker
#define PERMISSIONS1_PRIORITY_SPEAKER     (1 << 63)