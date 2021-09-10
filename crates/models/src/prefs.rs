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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Font {
    SansSerif,
    Serif,
    OpenDyslexic,
    Monospace,
    Cursive,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Hash, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UserPreference {
    Locale,

    /*
        PRIVACY
    */
    /// Allow DMs from server members
    AllowDms,
    /// Who can add you as a friend,
    /// number 0-3 where 0 = no one, 1 = friends of friends, 2 = server members, 3 = anyone
    FriendAdd,

    /*
        ACCESSIBILITY
    */
    /// Reduce animations
    ReduceAnim,
    /// Pause media/GIFs when window/tab is unfocused
    UnfocusPause,

    /*
        APPEARANCE
    */
    /// Light-theme toggle
    IsLight,
    /// Color temperature
    Temp,
    /// Compact chat view
    Compact,
    /// Chat font
    ChatFont,
    /// UI Font
    UiFont,
    /// Font size
    ChatFontSize,
    /// UI Font Size
    UIFontSize,
    /// Time format
    TimeFormat,
}

impl fmt::Display for UserPreference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use serde::Serialize;
        self.serialize(f)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferences(HashMap<UserPreference, Value>);

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
            Self::Locale => match value.as_u64() {
                Some(value) => {
                    kind = UserPreferenceErrorKind::InvalidValue;
                    value < Locale::__MAX_LOCALE as u64
                }
                _ => false,
            },
            Self::FriendAdd => match value.as_u64() {
                Some(value) => {
                    kind = UserPreferenceErrorKind::InvalidValue;
                    value <= 3
                }
                _ => false,
            },
            Self::AllowDms | Self::ReduceAnim | Self::UnfocusPause | Self::IsLight | Self::Compact => {
                value.is_boolean()
            }
            Self::Temp => value.is_u64(),
            Self::TimeFormat => match value {
                // TODO: Properly validate format string
                Value::String(_format) => true,
                Value::Bool(_) => true,
                _ => false,
            },
            Self::ChatFont | Self::UiFont => match value {
                Value::String(value) => {
                    kind = UserPreferenceErrorKind::InvalidValue;
                    serde_json::from_str::<Font>(value).is_ok()
                }
                _ => false,
            },
            Self::ChatFontSize | Self::UIFontSize => match value.as_f64() {
                Some(value) => {
                    kind = UserPreferenceErrorKind::InvalidValue;
                    value > 0.0
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
}
