use std::time::SystemTime;

pub fn leap_years_before(year: i32) -> i32 {
    year / 4 - year / 100 + year / 400
}

pub fn leap_years_between(start: i32, end: i32) -> i32 {
    leap_years_before(end) - leap_years_before(start)
}

pub fn is_of_age(min_age: i64, now: SystemTime, dob: time::Date) -> bool {
    let today = time::OffsetDateTime::from(now).date();

    let days = (today - dob).whole_days() - leap_years_between(dob.year(), today.year()) as i64;

    days >= min_age * 365
}
