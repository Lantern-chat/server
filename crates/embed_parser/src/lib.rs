pub mod embed;
pub mod html;
pub mod msg;
pub mod oembed;
pub mod req;

#[inline(always)]
fn is_quote(c: char) -> bool {
    c == '"' || c == '\''
}

fn trim_quotes(s: &str) -> &str {
    let mut start = 0;
    let mut end = s.len();

    for c in s.chars() {
        if !is_quote(c) {
            break;
        }

        start += 1;
    }

    for c in s.chars().rev() {
        if !is_quote(c) {
            break;
        }

        end -= 1;
    }

    &s[start..end]
}
