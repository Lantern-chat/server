use std::ops::RangeInclusive;

const BANNED_USERNAMES: &[&str] = &["SYSTEM", "Admin", "Administrator", "Webmaster"];

fn all_count<P>(value: &str, range: RangeInclusive<usize>, mut pred: P) -> bool
where
    P: FnMut(char) -> bool,
{
    let mut len = 0;

    let start = *range.end();
    let end = *range.end();

    if value.len() < start {
        return false;
    }

    for c in value.chars() {
        len += 1;

        if !pred(c) || len > end {
            return false;
        }
    }

    len > start
}

pub fn validate_name(name: &str, len: RangeInclusive<usize>) -> bool {
    if !all_count(name, len, |c| !c.is_ascii_control() && !c.is_whitespace()) {
        return false;
    }

    if crate::names::contains_bad_words(name) {
        return false;
    }

    true
}

pub fn validate_username(username: &str, len: RangeInclusive<usize>) -> bool {
    if !all_count(username, len, |c| !c.is_ascii_control() && !c.is_whitespace()) {
        return false;
    }

    for u in BANNED_USERNAMES {
        if u.eq_ignore_ascii_case(username) {
            return false;
        }
    }

    if crate::names::contains_bad_words(username) {
        return false;
    }

    true
}

/// Passwords must be within the length
pub fn validate_password(password: &str, len: RangeInclusive<usize>) -> bool {
    let start = *len.start();
    let end = *len.end();

    if password.len() < start {
        return false;
    }

    let mut len = 0;

    let mut has_char = false;
    let mut has_num = false;
    let mut has_special = false;

    for c in password.chars() {
        len += 1;

        if c.is_alphabetic() {
            has_char = true;
        } else if c.is_numeric() {
            has_num = true;
        } else if c.is_ascii_control() {
            return false;
        } else {
            has_special = true;
        }

        if len >= start && has_char && (has_num || has_special) {
            return true;
        }

        if len > end {
            return false;
        }
    }

    false
}

/// It's basically impossible to properly validate an email
/// other than to just send the email, so the best we can do is
/// check if it has an `@` symbol and is longer than 3 characters
pub fn validate_email(email: &str) -> bool {
    if email.len() > 2048 {
        return false;
    }

    let mut len = 0;
    let mut has_at = false;

    for c in email.chars() {
        len += 1;
        has_at |= c == '@';

        if has_at && len >= 3 {
            return true;
        }
    }

    false
}
