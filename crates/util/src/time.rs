use std::time::SystemTime;

use time::{Date, PrimitiveDateTime};
use timestamp::Timestamp;

#[inline]
fn leap_years_before1(year: i32) -> i32 {
    //year -= 1;
    year / 4 - year / 100 + year / 400
}

//pub const fn is_leap_year(year: i32) -> bool {
//    (year % 4 == 0) & ((year % 100 != 0) | (year % 400 == 0))
//}

// https://stackoverflow.com/a/4587611/2083075
#[inline]
pub fn leap_years_between(start: i32, end: i32) -> i32 {
    leap_years_before1(end - 1) - leap_years_before1(start)
}

fn is_of_age_inner(min_age: i64, ts: PrimitiveDateTime, dob: Date) -> bool {
    let days = (ts - dob.midnight()).whole_days() - leap_years_between(dob.year(), ts.year()) as i64;

    //println!("DAYS: {} >= {}", days + 1, min_age * 365);

    days >= min_age * 365
}

#[inline]
pub fn is_of_age(min_age: i64, ts: SystemTime, dob: Date) -> bool {
    is_of_age_inner(min_age, *Timestamp::from(ts), dob)
}

#[cfg(test)]
mod tests {
    use super::*;

    use time::macros::{date, datetime};

    #[test]
    fn test_age() {
        let min_age = 13;

        assert!(!is_of_age_inner(
            min_age,
            datetime!(2020 - 01 - 03 23:59),
            date!(2007 - 01 - 04)
        ));

        assert!(is_of_age_inner(
            min_age,
            datetime!(2020 - 01 - 04 00:00),
            date!(2007 - 01 - 04)
        ));

        assert!(!is_of_age_inner(
            min_age,
            datetime!(2017 - 02 - 27 23:59),
            date!(2004 - 02 - 29)
        ));

        assert!(is_of_age_inner(
            min_age,
            datetime!(2017 - 02 - 28 00:00),
            date!(2004 - 02 - 29)
        ));

        assert!(!is_of_age_inner(
            min_age,
            datetime!(2020 - 02 - 27 23:59),
            date!(2007 - 02 - 28)
        ));

        assert!(is_of_age_inner(
            min_age,
            datetime!(2020 - 02 - 28 00:00),
            date!(2007 - 02 - 28)
        ));
    }
}
