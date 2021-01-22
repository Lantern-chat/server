use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AuthToken(pub [u8; Self::TOKEN_LEN]);

impl AuthToken {
    pub const TOKEN_LEN: usize = 32;
}

impl AuthToken {
    pub fn as_str(&self) -> &str {
        // We've already checked it's valid ASCII below
        unsafe { std::str::from_utf8_unchecked(&self.0) }
    }
}

impl FromStr for AuthToken {
    // TODO: Better error type
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = s.as_bytes();
        if bytes.len() == Self::TOKEN_LEN && s.is_ascii() {
            let mut token = AuthToken([0; Self::TOKEN_LEN]);
            token.0.copy_from_slice(bytes);
            Ok(token)
        } else {
            Err(())
        }
    }
}
