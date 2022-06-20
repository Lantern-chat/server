use schema::auth::SplitBotToken;

#[inline(never)]
#[no_mangle]
pub fn bot_token_to_bytes(token: &SplitBotToken) -> [u8; 36] {
    token.to_bytes()
}

#[inline(never)]
#[no_mangle]
pub fn bot_token_from_bytes(bytes: [u8; 32]) -> SplitBotToken {
    SplitBotToken::try_from(&bytes[..]).unwrap()
}

fn main() {}
