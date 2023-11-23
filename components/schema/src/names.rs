pub fn slug_name(name: &str) -> String {
    let mut new_name = String::new();

    let mut last_was_ws = false;
    for c in name.trim().chars() {
        if c.is_whitespace() {
            if !last_was_ws {
                new_name.push_str("-");
            }

            last_was_ws = true;
        } else {
            last_was_ws = false;

            new_name.extend(c.to_lowercase());
        }
    }

    new_name
}

pub fn contains_bad_words(name: &str) -> bool {
    use rustrict::{Censor, Type};

    let c = Censor::from_str(name).analyze();

    let offensive = Type::OFFENSIVE & Type::MODERATE_OR_HIGHER;
    let extreme = (Type::PROFANE | Type::SEXUAL | Type::MEAN) & Type::SEVERE;
    let not_evasive = !(Type::EVASIVE & Type::MODERATE_OR_HIGHER);

    c.is((offensive | extreme) & not_evasive)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slug_name() {
        assert_eq!("testing-room", slug_name("   Testing\n   RoOM   \n"));
    }
}
