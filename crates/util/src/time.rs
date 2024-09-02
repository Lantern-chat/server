#![allow(unused)]

use std::time::SystemTime;

use time::{Date, PrimitiveDateTime};
use timestamp::Timestamp;

#[inline]
fn leap_years_before1(year: i32) -> i32 {
    //year -= 1;
    year / 4 - year / 100 + year / 400
}

const fn is_leap_year(y: i32) -> bool {
    //(y % 4 == 0) & ((y % 25 != 0) | (y % 16 == 0)) // old version

    // ternary compiles to cmov
    y & (if y % 25 == 0 { 15 } else { 3 }) == 0
}

// https://stackoverflow.com/a/4587611/2083075
#[inline]
pub fn leap_years_between(start: i32, end: i32) -> i32 {
    leap_years_before1(end - 1) - leap_years_before1(start)
}

//fn is_of_age_inner(min_age: i64, ts: PrimitiveDateTime, dob: Date) -> bool {
//    let today = ts.date();
//
//    let mut days = (today - dob).whole_days() - leap_years_between(dob.year(), today.year()) as i64;
//
//    let birthday = Date::from_ordinal_date(today.year(), dob.ordinal()).unwrap();
//
//    if today > birthday {
//        days -= 1;
//    }
//
//    println!("{} {} DAYS: {} >= {}", ts, dob, days + 1, min_age * 365);
//
//    days >= min_age * 365
//}

fn birthday(year: i32, dob: Date) -> Date {
    let mut ordinal = dob.ordinal();

    const FEB_28: u16 = 59; // 31 + 28
    const FEB_29: u16 = 60; // 31 + 29

    if ordinal > FEB_28 {
        let current_year_leap_year = is_leap_year(year);
        let birth_leap_year = is_leap_year(dob.year());

        if !current_year_leap_year && birth_leap_year && ordinal > FEB_29 {
            ordinal -= 1;
        }

        if current_year_leap_year && !birth_leap_year {
            ordinal += 1;
        }
    }

    Date::from_ordinal_date(year, ordinal).unwrap()
}

fn is_of_age_inner2(min_age: i32, ts: PrimitiveDateTime, dob: Date) -> bool {
    let today = ts.date();

    let birthday = birthday(today.year(), dob);

    let mut age = today.year() - dob.year();

    if today < birthday {
        age -= 1;
    }

    age >= min_age
}

fn is_of_age_inner(min_age: i32, ts: PrimitiveDateTime, dob: Date) -> bool {
    let today = ts.date();

    let mut years = today.year() - dob.year() - 1;

    if (today.month() as u8) > (dob.month() as u8) || (today.month() == dob.month() && today.day() >= dob.day()) {
        years += 1;
    }

    years >= min_age
}

#[inline]
pub fn is_of_age(min_age: i32, ts: SystemTime, dob: Date) -> bool {
    is_of_age_inner(min_age, *Timestamp::from(ts), dob)
}

#[cfg(test)]
mod tests {
    use super::*;

    use time::macros::{date, datetime};

    #[test]
    fn test_birthday() {
        fn tb(dob: Date, expected: Date) {
            assert_eq!(birthday(expected.year(), dob), expected, "{dob} -> {expected}");
        }

        // birthdays are leap years
        tb(date!(2004 - 02 - 29), date!(2020 - 02 - 29));
        tb(date!(2004 - 03 - 29), date!(2020 - 03 - 29));
        tb(date!(2004 - 02 - 23), date!(2020 - 02 - 23));
        tb(date!(2004 - 03 - 23), date!(2021 - 03 - 23));
        tb(date!(2004 - 02 - 28), date!(2021 - 02 - 28));
        tb(date!(2004 - 02 - 29), date!(2021 - 03 - 01));
        tb(date!(2004 - 03 - 01), date!(2021 - 03 - 01));
        tb(date!(2004 - 03 - 02), date!(2021 - 03 - 02));
        tb(date!(2004 - 03 - 03), date!(2021 - 03 - 03));

        // current year is leap year
        tb(date!(2005 - 03 - 01), date!(2020 - 03 - 01));
        tb(date!(2005 - 02 - 23), date!(2020 - 02 - 23));
        tb(date!(2005 - 02 - 28), date!(2020 - 02 - 28));
        tb(date!(2005 - 03 - 28), date!(2020 - 03 - 28));
        tb(date!(2005 - 02 - 23), date!(2020 - 02 - 23));
        tb(date!(2005 - 03 - 01), date!(2020 - 03 - 01));
        tb(date!(2005 - 03 - 02), date!(2020 - 03 - 02));

        // no leap years
        tb(date!(2005 - 03 - 28), date!(2021 - 03 - 28));
        tb(date!(2005 - 02 - 28), date!(2021 - 02 - 28));

        // regular dates
        tb(date!(2005 - 02 - 02), date!(2021 - 02 - 02));
        tb(date!(2005 - 07 - 28), date!(2021 - 07 - 28));
        tb(date!(2005 - 01 - 28), date!(2021 - 01 - 28));

        // mixed regular dates
        tb(date!(2004 - 02 - 02), date!(2021 - 02 - 02));
        tb(date!(2004 - 07 - 28), date!(2021 - 07 - 28));
        tb(date!(2004 - 01 - 28), date!(2021 - 01 - 28));
        tb(date!(2004 - 02 - 02), date!(2020 - 02 - 02));
        tb(date!(2004 - 07 - 28), date!(2020 - 07 - 28));
        tb(date!(2004 - 01 - 28), date!(2020 - 01 - 28));
    }

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
            datetime!(2017 - 02 - 28 23:59),
            date!(2004 - 02 - 29)
        ));

        assert!(is_of_age_inner(
            min_age,
            datetime!(2017 - 03 - 01 00:00),
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

        assert!(!is_of_age_inner(
            min_age,
            datetime!(2020 - 03 - 27 23:59),
            date!(2007 - 03 - 28)
        ));

        assert!(is_of_age_inner(
            min_age,
            datetime!(2020 - 03 - 28 00:00),
            date!(2007 - 03 - 28)
        ));

        assert!(is_of_age_inner(
            min_age,
            datetime!(2020 - 06 - 28 00:00),
            date!(2007 - 06 - 28)
        ));

        assert!(!is_of_age_inner(
            min_age,
            datetime!(2020 - 06 - 27 23:59),
            date!(2007 - 06 - 28)
        ));

        assert!(is_of_age_inner(
            min_age,
            datetime!(2020 - 06 - 26 23:59),
            date!(2007 - 06 - 25)
        ));
    }
}
