use std::time::SystemTime;

use hmac::{
    digest::{FixedOutput, Key},
    Mac, SimpleHmac,
};
use sha1::Sha1;

use sdk::models::{Snowflake, SplitBotToken};

type Sha1Hmac = SimpleHmac<Sha1>;
pub type BotTokenKey = Key<Sha1Hmac>;

pub trait SplitBotTokenExt {
    /// Generates a new bot token at this time
    fn new(key: &BotTokenKey, id: Snowflake) -> Self;
    fn verify(&self, key: &BotTokenKey) -> bool;
}

fn token_mac(token: &SplitBotToken, key: &BotTokenKey) -> Sha1Hmac {
    let mut mac = Sha1Hmac::new(key);
    mac.update(&token.to_bytes()[0..16]);
    mac
}

impl SplitBotTokenExt for SplitBotToken {
    fn new(key: &BotTokenKey, id: Snowflake) -> Self {
        let mut t = SplitBotToken {
            id,
            hmac: [0; 20],
            ts: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        t.hmac = token_mac(&t, key).finalize_fixed().into();

        t
    }

    fn verify(&self, key: &BotTokenKey) -> bool {
        token_mac(self, key).verify_slice(&self.hmac).is_ok()
    }
}
