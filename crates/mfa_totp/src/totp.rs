use std::str::FromStr;

use hmac::{
    digest::{FixedOutputReset, Key},
    Hmac, Mac,
};
use sha2::Sha256;

pub type HmacSha256 = Hmac<Sha256>;
pub type TOTPKey = Key<HmacSha256>;

pub type TOTP6<'a> = TOTP<'a, 6>;
pub type TOTP8<'a> = TOTP<'a, 8>;

pub struct TOTP<'a, const DIGITS: usize> {
    pub key: &'a [u8],
    pub step: u64,
}

impl<'a, const DIGITS: usize> TOTP<'a, DIGITS> {
    pub fn new(key: &'a [u8]) -> Self {
        assert!(key.len() >= (256 / 8), "TOTP Key should be at least 256-bit");
        TOTP { key, step: 30 }
    }

    /// https://datatracker.ietf.org/doc/html/rfc6238#appendix-A
    pub fn generate_raw(&self, hmac: &mut HmacSha256, step: u64) -> u32 {
        let hash = {
            hmac.update(&step.to_be_bytes());
            hmac.finalize_fixed_reset()
        };

        let binary = {
            // get last byte and use it as an index to read in a word
            let offset = (hash[hash.len() - 1] & 0xF) as usize;

            let mut buf = [0u8; 4];
            buf.copy_from_slice(&hash[offset..offset + 4]);

            0x7fff_ffff & u32::from_be_bytes(buf)
        };

        binary % 10u32.pow(DIGITS as u32)
    }

    pub fn generate_formatted(&self, time: u64) -> String {
        let mut hmac = HmacSha256::new_from_slice(self.key).unwrap();
        format!("{1:00$}", DIGITS, self.generate_raw(&mut hmac, time / self.step))
    }

    /// To avoid TOTP reuse, we must track the last-used time
    #[inline(always)]
    fn check_raw(&self, hmac: &mut HmacSha256, token: u32, step: u64, last: &mut u64) -> bool {
        if *last >= step {
            return false;
        }

        if self.generate_raw(hmac, step) == token {
            *last = step.max(*last);
            return true;
        }

        false
    }

    pub fn check(&self, token: u32, time: u64, last: &mut u64) -> bool {
        let step = time / self.step;

        if *last >= step {
            return false;
        }

        let mut hmac = HmacSha256::new_from_slice(self.key).unwrap();

        // no skew, most likely
        self.check_raw(&mut hmac, token, step, last) ||
        // skew backwards, second likely
        self.check_raw(&mut hmac, token, step - 1, last) ||
        // skew forward, weird
        self.check_raw(&mut hmac, token, step + 1, last)
    }

    pub fn check_str(&self, token: &str, time: u64, last: &mut u64) -> Result<bool, <u32 as FromStr>::Err> {
        if token.len() != DIGITS {
            return Ok(false);
        }

        Ok(self.check(token.parse()?, time, last))
    }

    pub fn url(&self, label: &str, issuer: &str) -> String {
        let secret = base32::encode(base32::Alphabet::RFC4648 { padding: false }, self.key);

        format!("otpauth://totp/{label}?secret={secret}&issuer={issuer}&digits={DIGITS}&algorithm=SHA256")
    }
}

#[cfg(test)]
mod tests {
    use std::time::SystemTime;

    use super::*;

    #[test]
    fn print_totps() {
        const TEST_TIMES: &[u64] = &[59, 1111111109, 1111111111, 1234567890, 2000000000, 20000000000];
        let key = hex::decode("3132333435363738393031323334353637383930313233343536373839303132").unwrap();

        println!("Keylen: {}", key.len());

        let totp = TOTP8::new(&key);

        for t in TEST_TIMES {
            println!("{}: {}", t, totp.generate_formatted(*t));
        }
    }

    #[test]
    fn test_now_totp() {
        let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();

        let key = hex::decode("3132333435363738393031323334353637383930313233343536373839303132").unwrap();

        println!("Keylen: {}", key.len());

        let totp = TOTP6::new(&key);

        println!("{}", totp.url("test", "testing"));
        println!("{}", totp.generate_formatted(now));

        //for t in 0..100000000 {
        //    assert_eq!(totp.generate_backup(t).len(), 13);
        //}
    }
}
