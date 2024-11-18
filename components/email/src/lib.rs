use smol_str::SmolStr;

#[macro_use]
mod macros;

pub mod fmt;
pub mod scenarios;
pub mod tpl_manager;

#[derive(ramhorns::Content)]
pub struct Email {
    pub to: SmolStr,
    pub subject: SmolStr,
    pub ts: timestamp::Timestamp,

    pub scenario: scenarios::Scenario,
}

impl Email {
    pub fn new(
        to: impl Into<SmolStr>,
        subject: impl Into<SmolStr>,
        scenario: impl Into<scenarios::Scenario>,
    ) -> Self {
        Self {
            to: to.into(),
            subject: subject.into(),
            scenario: scenario.into(),
            ts: timestamp::Timestamp::now_utc(),
        }
    }
}
