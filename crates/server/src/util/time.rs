use std::time::SystemTime;

use chrono::{Date, DateTime, Datelike, NaiveDate, NaiveDateTime, SecondsFormat, Utc};

pub fn leap_years_before(year: i32) -> i32 {
    year / 4 - year / 100 + year / 400
}

pub fn leap_years_between(start: i32, end: i32) -> i32 {
    leap_years_before(end) - leap_years_before(start)
}

pub fn is_of_age(min_age: i64, now: SystemTime, dob: NaiveDate) -> bool {
    let dob = Date::<Utc>::from_utc(dob, Utc);
    let today = DateTime::<Utc>::from(now).date();

    let days = (today - dob).num_days() - leap_years_between(dob.year(), today.year()) as i64;

    days >= min_age * 365
}

//pub fn format_naivedatetime(dt: NaiveDateTime) -> String {
//    DateTime::<Utc>::from_utc(dt, Utc).to_rfc3339_opts(SecondsFormat::Millis, true)
//}
