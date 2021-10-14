use std::collections::HashMap;
use std::fmt;

use serde_json::Value;

use super::*;

#[derive(Debug, Clone, Copy, Hash, serde_repr::Serialize_repr, serde_repr::Deserialize_repr)]
#[allow(non_camel_case_types)]
#[repr(u16)]
pub enum Locale {
    enUS = 0,

    __MAX_LOCALE,
}

impl Default for Locale {
    fn default() -> Locale {
        Locale::enUS
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, serde_repr::Serialize_repr, serde_repr::Deserialize_repr,
)]
#[repr(u16)]
#[allow(non_camel_case_types)]
pub enum Font {
    SansSerif = 0,
    Serif,
    Monospace,
    Cursive,
    ComicSans,

    // third-party fonts
    OpenDyslexic = 30,

    __MAX_FONT,
}

bitflags::bitflags! {
    pub struct UserPrefsFlags: i32 {
        /// Reduce movement and animations in the UI
        const REDUCE_ANIMATIONS = 1 << 0;
        /// Pause animations on window unfocus
        const UNFOCUS_PAUSE = 1 << 1;
        const LIGHT_MODE = 1 << 2;

        /// Allow direct messages from shared server memmbers
        const ALLOW_DMS = 1 << 3;
        /// Show small lines between message groups
        const GROUP_LINES = 1 << 4;
        const HIDE_AVATARS = 1 << 5;

        // give some space for other flags, and possibly switching compact view out for more options (multiple view types)
        const COMPACT_VIEW = 1 << 9;
        const DEVELOPER_MODE = 1 << 15;

        const DEFAULT_FLAGS = Self::ALLOW_DMS.bits | Self::GROUP_LINES.bits;
    }
}

serde_shims::impl_serde_for_bitflags!(UserPrefsFlags);

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Hash, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UserPreference {
    Locale,

    Flags,

    /*
        PRIVACY
    */
    /// Who can add you as a friend,
    /// number 0-3 where 0 = no one, 1 = friends of friends, 2 = server members, 3 = anyone
    FriendAdd,

    /*
        ACCESSIBILITY
    */

    /*
        APPEARANCE
    */
    /// Color temperature
    Temp,
    /// Chat font
    ChatFont,
    /// UI Font
    UiFont,
    /// Font size
    ChatFontSize,
    /// UI Font Size
    UIFontSize,
    /// Message Tab Size (in spaces)
    TabSize,
    /// Time format
    TimeFormat,
    /// Group padding
    Pad,

    /*
        Advanced
    */
    #[serde(other)]
    InvalidField,
}

impl fmt::Display for UserPreference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use serde::Serialize;
        self.serialize(f)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferences(HashMap<UserPreference, Value>);

#[derive(Debug, Clone, Copy)]
pub struct UserPreferenceError {
    pub field: UserPreference,
    pub kind: UserPreferenceErrorKind,
}

impl fmt::Display for UserPreferenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let kind = match self.kind {
            UserPreferenceErrorKind::InvalidType => "is invalid type",
            UserPreferenceErrorKind::InvalidValue => "has an invalid value",
        };
        write!(f, "User Preference Error: \"{}\" {}", self.field, kind)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserPreferenceErrorKind {
    InvalidType,
    InvalidValue,
}

impl UserPreferences {
    pub fn validate(&self) -> Result<(), UserPreferenceError> {
        for (field, value) in self.0.iter() {
            field.validate(value)?;
        }

        Ok(())
    }

    pub fn clean(&mut self) {
        self.0.retain(|field, value| field.validate(value).is_ok())
    }

    pub fn flags(&self) -> UserPrefsFlags {
        match self.0.get(&UserPreference::Flags).and_then(Value::as_u64) {
            Some(value) => UserPrefsFlags::from_bits_truncate(value as _),
            None => UserPrefsFlags::DEFAULT_FLAGS,
        }
    }

    pub fn nullify_defaults(&mut self) {
        let flags = self.flags();

        for (field, value) in self.0.iter_mut() {
            if field.is_default(value, flags) {
                *value = Value::Null;
            }
        }
    }

    pub fn merge(&mut self, new: &mut Self) {
        for (field, value) in new.0.drain() {
            self.0.insert(field, value);
        }
    }
}

impl UserPreference {
    pub fn validate(self, value: &Value) -> Result<(), UserPreferenceError> {
        let mut kind = UserPreferenceErrorKind::InvalidType;

        let valid_type = match self {
            // NULL values are not allowed
            _ if value.is_null() => false,

            Self::InvalidField => false,

            // The locale just has to be in the list of enums, and since
            // they are numbered it's easy to check
            Self::Locale => match value.as_u64() {
                Some(value) => {
                    kind = UserPreferenceErrorKind::InvalidValue;
                    value < Locale::__MAX_LOCALE as u64
                }
                _ => false,
            },
            // Check docs for this, but values can only be from 0-3 inclusive
            Self::FriendAdd => match value.as_u64() {
                Some(value) => {
                    kind = UserPreferenceErrorKind::InvalidValue;
                    value <= 3
                }
                _ => false,
            },
            Self::Flags => match value.as_u64() {
                Some(value) => {
                    kind = UserPreferenceErrorKind::InvalidValue;

                    // contained within 2^32 AND a valid flag
                    value <= (u32::MAX as u64) && UserPrefsFlags::from_bits(value as i32).is_some()
                }
                _ => false,
            },
            // Color temperature in kelvin degrees
            Self::Temp => match value.as_f64() {
                Some(temp) => {
                    kind = UserPreferenceErrorKind::InvalidValue;
                    965.0 <= temp && temp <= 12000.0
                }
                _ => false,
            },
            Self::TimeFormat => match value {
                // TODO: Properly validate format string
                Value::String(_format) => true,
                Value::Bool(_) => true,
                _ => false,
            },
            // Fonts must be in the list, which is easily checked by parsing the enum
            Self::ChatFont | Self::UiFont => match value.as_u64() {
                Some(value) => {
                    kind = UserPreferenceErrorKind::InvalidValue;
                    value < Font::__MAX_FONT as u64
                }
                _ => false,
            },
            Self::TabSize => match value.as_u64() {
                Some(value) => {
                    kind = UserPreferenceErrorKind::InvalidValue;
                    value > 0 && value < 64
                }
                _ => false,
            },
            // Font sizes can be floats for smooth scaling, but must be positive
            Self::ChatFontSize | Self::UIFontSize => match value.as_f64() {
                Some(value) => {
                    kind = UserPreferenceErrorKind::InvalidValue;
                    value > 0.0
                }
                _ => false,
            },
            Self::Pad => match value.as_u64() {
                Some(value) => {
                    kind = UserPreferenceErrorKind::InvalidValue;
                    value <= 32
                }
                _ => false,
            },
        };

        if !valid_type {
            Err(UserPreferenceError { field: self, kind })
        } else {
            Ok(())
        }
    }

    fn is_default(self, value: &Value, flags: UserPrefsFlags) -> bool {
        match self {
            Self::Flags => value.as_u64() == Some(UserPrefsFlags::DEFAULT_FLAGS.bits() as u64),
            Self::ChatFontSize | Self::UIFontSize => value.as_f64() == Some(1.0),
            Self::Temp => value.as_f64() == Some(7500.0),
            Self::FriendAdd => value.as_u64() == Some(3),
            Self::Locale => value.as_u64() == Some(Locale::enUS as u64),
            Self::ChatFont | Self::UiFont => value.as_u64() == Some(0),
            Self::TabSize => value.as_u64() == Some(4),
            Self::Pad => {
                let value = value.as_u64();

                if flags.contains(UserPrefsFlags::COMPACT_VIEW) {
                    value == Some(0)
                } else {
                    value == Some(16)
                }
            }
            _ => false,
        }
    }
}
