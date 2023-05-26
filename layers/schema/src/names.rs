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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slug_name() {
        assert_eq!("testing-room", slug_name("   Testing\n   RoOM   \n"));
    }
}
