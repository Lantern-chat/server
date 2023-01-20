
----------------------------------------
---------- USER PREFS FLAGS ------------
----------------------------------------

-- Reduce movement and animations in the UI
#define USER_PREFS_REDUCE_ANIMATIONS         (1 << 0)
-- Pause animations on window unfocus
#define USER_PREFS_UNFOCUS_PAUSE             (1 << 1)
#define USER_PREFS_LIGHT_MODE                (1 << 2)
-- Allow direct messages from shared server memmbers
#define USER_PREFS_ALLOW_DMS                 (1 << 3)
-- Show small lines between message groups
#define USER_PREFS_GROUP_LINES               (1 << 4)
#define USER_PREFS_HIDE_AVATARS              (1 << 5)
-- Display dark theme in an OLED-compatible mode
#define USER_PREFS_OLED_MODE                 (1 << 6)
-- Mute videos/audio by default
#define USER_PREFS_MUTE_MEDIA                (1 << 7)
-- Hide images/video with unknown dimensions
#define USER_PREFS_HIDE_UNKNOWN_DIMENSIONS   (1 << 8)
#define USER_PREFS_COMPACT_VIEW              (1 << 9)
-- Prefer browser/platform emojis rather than twemoji
#define USER_PREFS_USE_PLATFORM_EMOJIS       (1 << 10)
#define USER_PREFS_ENABLE_SPELLCHECK         (1 << 11)
#define USER_PREFS_LOW_BANDWIDTH_MODE        (1 << 12)
#define USER_PREFS_FORCE_COLOR_CONSTRAST     (1 << 13)
-- Displays information like mime type and file size
#define USER_PREFS_SHOW_MEDIA_METADATA       (1 << 14)
#define USER_PREFS_DEVELOPER_MODE            (1 << 15)
#define USER_PREFS_SHOW_DATE_CHANGE          (1 << 16)
#define USER_PREFS_HIDE_LAST_ACTIVE          (1 << 17)

----------------------------------------
------------ MEMBER FLAGS --------------
----------------------------------------

#define MEMBER_BANNED 1

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

#define MESSAGE_DELETED 1

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
