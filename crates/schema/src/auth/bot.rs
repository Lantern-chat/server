use hmac::{digest::Key, Mac, SimpleHmac};
use sha1::Sha1;

type Sha1Hmac = SimpleHmac<Sha1>;

pub type BotTokenKey = Key<Sha1Hmac>;

use sdk::models::{Snowflake, SplitBotToken};

pub trait SplitBotTokenExt {
    fn new(key: &BotTokenKey, id: Snowflake, ts: u64) -> Self;
    fn verify(&self, key: &BotTokenKey) -> bool;
}

impl SplitBotTokenExt for SplitBotToken {
    fn new(key: &BotTokenKey, id: Snowflake, ts: u64) -> Self {
        let mut t = SplitBotToken {
            id,
            ts,
            hmac: [0; 20],
        };

        t.hmac = {
            let mut mac = Sha1Hmac::new(key);
            mac.update(&t.to_bytes()[0..16]);
            mac.finalize().into_bytes().into()
        };

        t
    }

    fn verify(&self, key: &BotTokenKey) -> bool {
        let mut mac = Sha1Hmac::new(key);
        mac.update(&self.to_bytes()[0..16]);
        mac.verify_slice(&self.hmac).is_ok()
    }
}
