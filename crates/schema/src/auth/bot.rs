use hmac::{digest::Key, Mac, SimpleHmac};
use sha1::Sha1;

type Sha1Hmac = SimpleHmac<Sha1>;

pub type BotTokenKey = Key<Sha1Hmac>;

use sdk::models::SplitBotToken;

pub trait SplitBotTokenExt {
    fn verify(&self, key: &BotTokenKey) -> bool;
    fn compute_hmac(&self, key: &BotTokenKey) -> [u8; 20];
}

impl SplitBotTokenExt for SplitBotToken {
    fn verify(&self, key: &BotTokenKey) -> bool {
        let mut mac = Sha1Hmac::new(key);
        mac.update(&self.to_bytes()[0..16]);
        mac.verify_slice(&self.hmac).is_ok()
    }

    fn compute_hmac(&self, key: &BotTokenKey) -> [u8; 20] {
        let mut mac = Sha1Hmac::new(key);
        mac.update(&self.to_bytes()[0..16]);
        mac.finalize().into_bytes().into()
    }
}
