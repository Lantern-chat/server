use std::{ops::RangeInclusive, time::Duration};

use sdk::models::Timestamp;
use smol_str::SmolStr;
use uuid::Uuid;

#[derive(Clone)]
pub struct SharedConfig {
    // Config settings
    pub config_id: Uuid,
    pub config_name: SmolStr,
    pub last_updated: Timestamp,

    // General settings
    pub server_name: SmolStr,

    // Web settings
    pub base_domain: SmolStr,
    pub cdn_domain: SmolStr,
    pub strict_cdn: bool,
    pub secure_web: bool,
    pub camo_enable: bool,
    pub fs_cache_interval: Duration,
    pub fs_cache_max_age: Duration,

    // Account settings
    pub session_duration: Duration,
    pub minimum_age: u8,
    pub password_length: RangeInclusive<usize>,
    pub username_length: RangeInclusive<usize>,
    pub mfa_backup_count: u8,
    pub mfa_pending_time: Duration,

    // User settings
    pub relative_time_random_factor: f32,
    pub max_status_length: usize,
    pub max_bio_length: usize,

    // Party settings
    pub party_name_length: RangeInclusive<usize>,
    pub party_description_length: RangeInclusive<usize>,
    pub room_name_length: RangeInclusive<usize>,
    pub room_topic_length: RangeInclusive<usize>,
    pub role_name_length: RangeInclusive<usize>,
    pub role_description_length: RangeInclusive<usize>,
    pub max_active_rooms: u16,
    pub max_total_rooms: u16,

    // Message settings
    pub max_newlines: u8,
    pub message_length: RangeInclusive<usize>,
    pub max_embeds: u8,
    pub max_regex_search_len: usize,

    // Upload settings
    pub max_upload_size: u64,
    pub max_upload_chunk: u32,
    pub orphan_cleanup: Duration,
    pub max_avatar_size: u32,
    pub max_banner_size: u32,
    pub avatar_width: u32,
    pub banner_width: u32,
    pub banner_height: u32,
    pub max_avatar_pixels: u32,
    pub max_banner_pixels: u32,

    // Service settings
    pub hcaptcha_secret: SmolStr,
    pub hcaptcha_sitekey: SmolStr,
    pub b2_app: SmolStr,
    pub b2_key: SmolStr,
    pub embed_worker_uris: Vec<SmolStr>,
}

use postgres_range::{BoundType, Range as PgRange, RangeBound};

impl SharedConfig {
    pub async fn save(&self, obj: &db::pool::Object) -> Result<Self, db::pool::Error> {
        #[inline]
        fn range(range: &RangeInclusive<usize>) -> PgRange<i64> {
            PgRange::new(
                Some(RangeBound::new(*range.start() as i64, BoundType::Inclusive)),
                Some(RangeBound::new(*range.end() as i64, BoundType::Inclusive)),
            )
        }

        #[inline]
        fn dur(dur: Duration) -> i64 {
            dur.as_millis() as i64
        }

        let fs_cache_interval = dur(self.fs_cache_interval);
        let fs_cache_max_age = dur(self.fs_cache_max_age);
        let session_duration = dur(self.session_duration);
        let mfa_pending_time = dur(self.mfa_pending_time);
        let orphan_cleanup = dur(self.orphan_cleanup);

        let password_length = range(&self.password_length);
        let username_length = range(&self.username_length);
        let party_name_length = range(&self.party_name_length);
        let party_description_length = range(&self.party_description_length);
        let room_name_length = range(&self.room_name_length);
        let room_topic_length = range(&self.room_topic_length);
        let role_name_length = range(&self.role_name_length);
        let role_description_length = range(&self.role_description_length);
        let message_length = range(&self.message_length);

        let row = obj.query_one2(crate::sql! {
            UPDATE Configs SET
                Configs./ConfigName         = #{&self.config_name as Configs::ConfigName},
                Configs./LastUpdated        = now(),
                Configs./ServerName         = #{&self.server_name as Configs::ServerName},
                Configs./CdnDomain          = #{&self.cdn_domain as Configs::CdnDomain},
                Configs./StrictCdn          = #{&self.strict_cdn as Configs::StrictCdn},
                Configs./BaseDomain         = #{&self.base_domain as Configs::BaseDomain},
                Configs./SecureWeb          = #{&self.secure_web as Configs::SecureWeb},
                Configs./CamoEnable         = #{&self.camo_enable as Configs::CamoEnable},
                Configs./FsCacheInterval    = #{&fs_cache_interval as Configs::FsCacheInterval},
                Configs./FsCacheMaxAge      = #{&fs_cache_max_age as Configs::FsCacheMaxAge},
                Configs./SessionDuration    = #{&session_duration as Configs::SessionDuration},
                Configs./MinimumAge         = ,
                Configs./PasswordLength     = #{&password_length as Configs::PasswordLength},
                Configs./UsernameLength     = #{&username_length as Configs::UsernameLength},
                Configs./MfaBackupCount     = ,
                Configs./MfaPendingTime     = #{&mfa_pending_time as Configs::MfaPendingTime},
                Configs./ReltimeRndFactor   = #{&self.relative_time_random_factor as Configs::ReltimeRndFactor},
                Configs./MaxStatusLen       = ,
                Configs./MaxBioLen          = ,
                Configs./PartyNameLen       = #{&party_name_length as Configs::PartyNameLen},
                Configs./PartyDescLen       = #{&party_description_length as Configs::PartyDescLen},
                Configs./RoomNameLen        = #{&room_name_length as Configs::RoomNameLen},
                Configs./RoomTopicLen       = #{&room_topic_length as Configs::RoomTopicLen},
                Configs./RoleNameLen        = #{&role_name_length as Configs::RoleNameLen},
                Configs./RoleDescLen        = #{&role_description_length as Configs::RoleDescLen},
                Configs./MaxActiveRooms     = ,
                Configs./MaxTotalRooms      = ,
                Configs./MaxNewlines        = ,
                Configs./MessageLength      = #{&message_length as Configs::MessageLength},
                Configs./MaxEmbeds          = ,
                Configs./RegexSearchLen     = ,
                Configs./MaxUploadSize      = ,
                Configs./MaxUploadChunk     = ,
                Configs./OrphanCleanup      = #{&orphan_cleanup as Configs::OrphanCleanup},
                Configs./MaxAvatarSize      = ,
                Configs./MaxBannerSize      = ,
                Configs./AvatarWidth        = ,
                Configs./BannerWidth        = ,
                Configs./BannerHeight       = ,
                Configs./MaxAvatarPixels    = ,
                Configs./MaxBannerPixels    = ,
                Configs./HcaptchaSecret     = ,
                Configs./HcaptchaSitekey    = ,
                Configs./B2App              = ,
                Configs./B2Key              = ,
                Configs./EmbedWorkerUris    =
            WHERE Configs.ConfigId = #{&self.config_id as Configs::ConfigId}
            RETURNING Configs.LastUpdated AS @LastUpdated
        })
        .await?;

        let mut config = self.clone();
        config.last_updated = row.last_updated()?;

        Ok(config)
    }

    pub async fn load(obj: &db::pool::Object) -> Result<Self, db::pool::Error> {
        #[rustfmt::skip]
        let row = obj.query_one2(crate::sql! {
            SELECT
                Configs.ConfigId            AS @_,
                Configs.ConfigName          AS @_,
                Configs.LastUpdated         AS @_,
                Configs.ServerName          AS @_,
                Configs.CdnDomain           AS @_,
                Configs.StrictCdn           AS @_,
                Configs.BaseDomain          AS @_,
                Configs.SecureWeb           AS @_,
                Configs.CamoEnable          AS @_,
                Configs.FsCacheInterval     AS @_,
                Configs.FsCacheMaxAge       AS @_,
                Configs.SessionDuration     AS @_,
                Configs.MinimumAge          AS @_,
                Configs.PasswordLength      AS @_,
                Configs.UsernameLength      AS @_,
                Configs.MfaBackupCount      AS @_,
                Configs.MfaPendingTime      AS @_,
                Configs.ReltimeRndFactor    AS @_,
                Configs.MaxStatusLen        AS @_,
                Configs.MaxBioLen           AS @_,
                Configs.PartyNameLen        AS @_,
                Configs.PartyDescLen        AS @_,
                Configs.RoomNameLen         AS @_,
                Configs.RoomTopicLen        AS @_,
                Configs.RoleNameLen         AS @_,
                Configs.RoleDescLen         AS @_,
                Configs.MaxActiveRooms      AS @_,
                Configs.MaxTotalRooms       AS @_,
                Configs.MaxNewlines         AS @_,
                Configs.MessageLength       AS @_,
                Configs.MaxEmbeds           AS @_,
                Configs.RegexSearchLen      AS @_,
                Configs.MaxUploadSize       AS @_,
                Configs.MaxUploadChunk      AS @_,
                Configs.OrphanCleanup       AS @_,
                Configs.MaxAvatarSize       AS @_,
                Configs.MaxBannerSize       AS @_,
                Configs.AvatarWidth         AS @_,
                Configs.BannerWidth         AS @_,
                Configs.BannerHeight        AS @_,
                Configs.MaxAvatarPixels     AS @_,
                Configs.MaxBannerPixels     AS @_,
                Configs.HcaptchaSecret      AS @_,
                Configs.HcaptchaSitekey     AS @_,
                Configs.B2App               AS @_,
                Configs.B2Key               AS @_,
                Configs.EmbedWorkerUris     AS @_
            FROM Configs LIMIT 1
        }).await?;

        fn dur(ms: i64) -> Duration {
            Duration::from_millis(ms.max(0) as u64)
        }

        fn range(range: PgRange<i32>) -> RangeInclusive<usize> {
            let (Some(lower), Some(upper)) = (range.lower(), range.upper()) else {
                panic!("Invalid Postgres Range: {:?}", range);
            };

            (lower.value as usize)..=(upper.value as usize)
        }

        Ok(SharedConfig {
            config_id: row.configs_config_id()?,
            config_name: row.configs_config_name()?,
            last_updated: row.configs_last_updated()?,
            server_name: row.configs_server_name()?,
            base_domain: row.configs_base_domain()?,
            cdn_domain: row.configs_cdn_domain()?,
            strict_cdn: row.configs_strict_cdn()?,
            secure_web: row.configs_secure_web()?,
            camo_enable: row.configs_camo_enable()?,
            fs_cache_interval: dur(row.configs_fs_cache_interval()?),
            fs_cache_max_age: dur(row.configs_fs_cache_max_age()?),
            session_duration: dur(row.configs_session_duration()?),
            minimum_age: row.configs_minimum_age::<i16>()? as u8,
            password_length: range(row.configs_password_length()?),
            username_length: range(row.configs_username_length()?),
            mfa_backup_count: row.configs_mfa_backup_count::<i16>()? as u8,
            mfa_pending_time: dur(row.configs_mfa_pending_time()?),
            relative_time_random_factor: row.configs_reltime_rnd_factor()?,
            max_status_length: row.configs_max_status_len::<i16>()? as usize,
            max_bio_length: row.configs_max_bio_len::<i16>()? as usize,
            party_name_length: range(row.configs_party_name_len()?),
            party_description_length: range(row.configs_party_desc_len()?),
            room_name_length: range(row.configs_room_name_len()?),
            room_topic_length: range(row.configs_room_topic_len()?),
            role_name_length: range(row.configs_role_name_len()?),
            role_description_length: range(row.configs_role_desc_len()?),
            max_active_rooms: row.configs_max_active_rooms::<i16>()? as u16,
            max_total_rooms: row.configs_max_total_rooms::<i16>()? as u16,
            max_newlines: row.configs_max_newlines::<i16>()? as u8,
            message_length: range(row.configs_message_length()?),
            max_embeds: row.configs_max_embeds::<i16>()? as u8,
            max_regex_search_len: row.configs_regex_search_len::<i64>()? as usize,
            max_upload_size: row.configs_max_upload_size::<i64>()? as u64,
            max_upload_chunk: row.configs_max_upload_chunk::<i32>()? as u32,
            orphan_cleanup: dur(row.configs_orphan_cleanup()?),
            max_avatar_size: row.configs_max_avatar_size()?,
            max_banner_size: row.configs_max_banner_size()?,
            avatar_width: row.configs_avatar_width()?,
            banner_width: row.configs_banner_width()?,
            banner_height: row.configs_banner_height()?,
            max_avatar_pixels: row.configs_max_avatar_pixels()?,
            max_banner_pixels: row.configs_max_banner_pixels()?,
            hcaptcha_secret: row.configs_hcaptcha_secret()?,
            hcaptcha_sitekey: row.configs_hcaptcha_sitekey()?,
            b2_app: row.configs_b2_app()?,
            b2_key: row.configs_b2_key()?,
            embed_worker_uris: row.configs_embed_worker_uris()?,
        })
    }
}
