use std::time::SystemTime;

pub fn is_of_age(min_age: i64, now: SystemTime, dob: time::Date) -> bool {
    let today = time::OffsetDateTime::from(now).date();
    let diff = today - dob;

    // TODO: Implement something better
    let mut days = diff.whole_days();

    // rough approximiation, if it's less than this, it'll be less than the exact
    if days < min_age * 365 {
        false
    } else {
        let mut age = 0;
        let mut year = today.year();
        let mut of_age;

        loop {
            year -= 1;
            days -= time::days_in_year(year) as i64;
            age += 1;
            of_age = age >= min_age;

            if days < 0 || of_age {
                break;
            }
        }

        of_age
    }
}
