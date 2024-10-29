use std::{borrow::Cow, ops::RangeInclusive, time::Duration};

use sdk::models::{HCaptchaSiteKey, Timestamp};
use smol_str::SmolStr;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error(transparent)]
    DbError(#[from] db::Error),

    #[error("Invalid HCaptcha Site Key")]
    InvalidHCaptchaSiteKey,
}

impl From<db::pg::Error> for ConfigError {
    fn from(e: db::pg::Error) -> Self {
        ConfigError::DbError(e.into())
    }
}

#[derive(Clone, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
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
    pub presence_timeout: Duration,

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
    pub hcaptcha_sitekey: HCaptchaSiteKey,
    pub b2_app: SmolStr,
    pub b2_key: SmolStr,
    pub embed_worker_uris: Vec<SmolStr>,
}

const _: () = {
    const fn assert_archive<T: rkyv::Archive>() {}
    assert_archive::<SharedConfig>();
};

use postgres_range::{BoundType, Range as PgRange, RangeBound};

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum ConfigIdentifier {
    /// Use the first config in the database
    #[default]
    First,

    /// Use the config with the given name
    ByName(Cow<'static, str>),

    /// Use the config with the given UUID
    ById(Uuid),
}

impl SharedConfig {
    pub async fn save(&self, obj: &db::Object) -> Result<Self, ConfigError> {
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
        let presence_timeout = dur(self.presence_timeout);
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

        let minimum_age = self.minimum_age as i16;
        let mfa_backup_count = self.mfa_backup_count as i16;
        let max_status_len = self.max_status_length as i16;
        let max_bio_len = self.max_bio_length as i16;
        let max_active_rooms = self.max_active_rooms as i16;
        let max_total_rooms = self.max_total_rooms as i16;
        let max_newlines = self.max_newlines as i16;
        let max_embeds = self.max_embeds as i16;
        let regex_search_len = self.max_regex_search_len as i16;

        let max_upload_size = self.max_upload_size as i64;
        let max_upload_chunk = self.max_upload_chunk as i32;

        let max_avatar_size = self.max_avatar_size as i32;
        let max_banner_size = self.max_banner_size as i32;
        let avatar_width = self.avatar_width as i32;
        let banner_width = self.banner_width as i32;
        let banner_height = self.banner_height as i32;
        let max_avatar_pixels = self.max_avatar_pixels as i32;
        let max_banner_pixels = self.max_banner_pixels as i32;

        let hcaptcha_sitekey = self.hcaptcha_sitekey.as_str();

        #[rustfmt::skip]
        let row = obj.query_one2(crate::sql! {
            UPDATE Config SET
                Config./ConfigName         = #{&self.config_name as Config::ConfigName},
                Config./LastUpdated        = now(),
                Config./ServerName         = #{&self.server_name as Config::ServerName},
                Config./CdnDomain          = #{&self.cdn_domain as Config::CdnDomain},
                Config./StrictCdn          = #{&self.strict_cdn as Config::StrictCdn},
                Config./BaseDomain         = #{&self.base_domain as Config::BaseDomain},
                Config./SecureWeb          = #{&self.secure_web as Config::SecureWeb},
                Config./CamoEnable         = #{&self.camo_enable as Config::CamoEnable},
                Config./FsCacheInterval    = #{&fs_cache_interval as Config::FsCacheInterval},
                Config./FsCacheMaxAge      = #{&fs_cache_max_age as Config::FsCacheMaxAge},
                Config./SessionDuration    = #{&session_duration as Config::SessionDuration},
                Config./MinimumAge         = #{&minimum_age as Config::MinimumAge},
                Config./PasswordLength     = #{&password_length as Config::PasswordLength},
                Config./UsernameLength     = #{&username_length as Config::UsernameLength},
                Config./MfaBackupCount     = #{&mfa_backup_count as Config::MfaBackupCount},
                Config./MfaPendingTime     = #{&mfa_pending_time as Config::MfaPendingTime},
                Config./ReltimeRndFactor   = #{&self.relative_time_random_factor as Config::ReltimeRndFactor},
                Config./MaxStatusLen       = #{&max_status_len as Config::MaxStatusLen},
                Config./MaxBioLen          = #{&max_bio_len as Config::MaxBioLen},
                Config./PresenceTimeout    = #{&presence_timeout as Config::PresenceTimeout},
                Config./PartyNameLen       = #{&party_name_length as Config::PartyNameLen},
                Config./PartyDescLen       = #{&party_description_length as Config::PartyDescLen},
                Config./RoomNameLen        = #{&room_name_length as Config::RoomNameLen},
                Config./RoomTopicLen       = #{&room_topic_length as Config::RoomTopicLen},
                Config./RoleNameLen        = #{&role_name_length as Config::RoleNameLen},
                Config./RoleDescLen        = #{&role_description_length as Config::RoleDescLen},
                Config./MaxActiveRooms     = #{&max_active_rooms as Config::MaxActiveRooms},
                Config./MaxTotalRooms      = #{&max_total_rooms as Config::MaxTotalRooms},
                Config./MaxNewlines        = #{&max_newlines as Config::MaxNewlines},
                Config./MessageLength      = #{&message_length as Config::MessageLength},
                Config./MaxEmbeds          = #{&max_embeds as Config::MaxEmbeds},
                Config./RegexSearchLen     = #{&regex_search_len as Config::RegexSearchLen},
                Config./MaxUploadSize      = #{&max_upload_size as Config::MaxUploadSize},
                Config./MaxUploadChunk     = #{&max_upload_chunk as Config::MaxUploadChunk},
                Config./OrphanCleanup      = #{&orphan_cleanup as Config::OrphanCleanup},
                Config./MaxAvatarSize      = #{&max_avatar_size as Config::MaxAvatarSize},
                Config./MaxBannerSize      = #{&max_banner_size as Config::MaxBannerSize},
                Config./AvatarWidth        = #{&avatar_width as Config::AvatarWidth},
                Config./BannerWidth        = #{&banner_width as Config::BannerWidth},
                Config./BannerHeight       = #{&banner_height as Config::BannerHeight},
                Config./MaxAvatarPixels    = #{&max_avatar_pixels as Config::MaxAvatarPixels},
                Config./MaxBannerPixels    = #{&max_banner_pixels as Config::MaxBannerPixels},
                Config./HcaptchaSecret     = #{&self.hcaptcha_secret as Config::HcaptchaSecret},
                Config./HcaptchaSitekey    = #{&hcaptcha_sitekey as Config::HcaptchaSitekey},
                Config./B2App              = NULLIF(#{&self.b2_app as Config::B2App}, ""),
                Config./B2Key              = NULLIF(#{&self.b2_key as Config::B2Key}, ""),
                Config./EmbedWorkerUris    = #{&self.embed_worker_uris as Config::EmbedWorkerUris}
            WHERE Config.ConfigId = #{&self.config_id as Config::ConfigId}
            RETURNING Config.LastUpdated AS @LastUpdated
        })
        .await?;

        let mut config = self.clone();
        config.last_updated = row.last_updated()?;

        Ok(config)
    }

    pub async fn load(obj: &db::Object, by: ConfigIdentifier) -> Result<Self, ConfigError> {
        #[rustfmt::skip]
        let row = obj.query_one2(crate::sql! {
            SELECT
                Config.ConfigId            AS @_,
                Config.ConfigName          AS @_,
                Config.LastUpdated         AS @_,
                Config.ServerName          AS @_,
                Config.CdnDomain           AS @_,
                Config.StrictCdn           AS @_,
                Config.BaseDomain          AS @_,
                Config.SecureWeb           AS @_,
                Config.CamoEnable          AS @_,
                Config.FsCacheInterval     AS @_,
                Config.FsCacheMaxAge       AS @_,
                Config.SessionDuration     AS @_,
                Config.MinimumAge          AS @_,
                Config.PasswordLength      AS @_,
                Config.UsernameLength      AS @_,
                Config.MfaBackupCount      AS @_,
                Config.MfaPendingTime      AS @_,
                Config.ReltimeRndFactor    AS @_,
                Config.MaxStatusLen        AS @_,
                Config.MaxBioLen           AS @_,
                Config.PresenceTimeout     AS @_,
                Config.PartyNameLen        AS @_,
                Config.PartyDescLen        AS @_,
                Config.RoomNameLen         AS @_,
                Config.RoomTopicLen        AS @_,
                Config.RoleNameLen         AS @_,
                Config.RoleDescLen         AS @_,
                Config.MaxActiveRooms      AS @_,
                Config.MaxTotalRooms       AS @_,
                Config.MaxNewlines         AS @_,
                Config.MessageLength       AS @_,
                Config.MaxEmbeds           AS @_,
                Config.RegexSearchLen      AS @_,
                Config.MaxUploadSize       AS @_,
                Config.MaxUploadChunk      AS @_,
                Config.OrphanCleanup       AS @_,
                Config.MaxAvatarSize       AS @_,
                Config.MaxBannerSize       AS @_,
                Config.AvatarWidth         AS @_,
                Config.BannerWidth         AS @_,
                Config.BannerHeight        AS @_,
                Config.MaxAvatarPixels     AS @_,
                Config.MaxBannerPixels     AS @_,
                Config.HcaptchaSecret      AS @_,
                Config.HcaptchaSitekey     AS @_,
                Config.B2App               AS @_,
                Config.B2Key               AS @_,
                Config.EmbedWorkerUris     AS @_
            FROM Config

            match &by {
                ConfigIdentifier::First => {},
                ConfigIdentifier::ByName(name) => { WHERE Config.ConfigName = #{name as Config::ConfigName} },
                ConfigIdentifier::ById(id) => { WHERE Config.ConfigId = #{id as Config::ConfigId} },
            }

            LIMIT 1
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
            config_id: row.config_config_id()?,
            config_name: row.config_config_name()?,
            last_updated: row.config_last_updated()?,
            server_name: row.config_server_name()?,
            base_domain: row.config_base_domain()?,
            cdn_domain: row.config_cdn_domain()?,
            strict_cdn: row.config_strict_cdn()?,
            secure_web: row.config_secure_web()?,
            camo_enable: row.config_camo_enable()?,
            fs_cache_interval: dur(row.config_fs_cache_interval()?),
            fs_cache_max_age: dur(row.config_fs_cache_max_age()?),
            session_duration: dur(row.config_session_duration()?),
            minimum_age: row.config_minimum_age::<i16>()? as u8,
            password_length: range(row.config_password_length()?),
            username_length: range(row.config_username_length()?),
            mfa_backup_count: row.config_mfa_backup_count::<i16>()? as u8,
            mfa_pending_time: dur(row.config_mfa_pending_time()?),
            relative_time_random_factor: row.config_reltime_rnd_factor()?,
            max_status_length: row.config_max_status_len::<i16>()? as usize,
            max_bio_length: row.config_max_bio_len::<i16>()? as usize,
            presence_timeout: dur(row.config_presence_timeout()?),
            party_name_length: range(row.config_party_name_len()?),
            party_description_length: range(row.config_party_desc_len()?),
            room_name_length: range(row.config_room_name_len()?),
            room_topic_length: range(row.config_room_topic_len()?),
            role_name_length: range(row.config_role_name_len()?),
            role_description_length: range(row.config_role_desc_len()?),
            max_active_rooms: row.config_max_active_rooms::<i16>()? as u16,
            max_total_rooms: row.config_max_total_rooms::<i16>()? as u16,
            max_newlines: row.config_max_newlines::<i16>()? as u8,
            message_length: range(row.config_message_length()?),
            max_embeds: row.config_max_embeds::<i16>()? as u8,
            max_regex_search_len: row.config_regex_search_len::<i64>()? as usize,
            max_upload_size: row.config_max_upload_size::<i64>()? as u64,
            max_upload_chunk: row.config_max_upload_chunk::<i32>()? as u32,
            orphan_cleanup: dur(row.config_orphan_cleanup()?),
            max_avatar_size: row.config_max_avatar_size()?,
            max_banner_size: row.config_max_banner_size()?,
            avatar_width: row.config_avatar_width()?,
            banner_width: row.config_banner_width()?,
            banner_height: row.config_banner_height()?,
            max_avatar_pixels: row.config_max_avatar_pixels()?,
            max_banner_pixels: row.config_max_banner_pixels()?,
            hcaptcha_secret: row.config_hcaptcha_secret()?,
            hcaptcha_sitekey: HCaptchaSiteKey::try_from(row.config_hcaptcha_sitekey::<&str>()?)
                .ok_or(ConfigError::InvalidHCaptchaSiteKey)?,
            b2_app: row.config_b2_app()?,
            b2_key: row.config_b2_key()?,
            embed_worker_uris: row.config_embed_worker_uris()?,
        })
    }
}
