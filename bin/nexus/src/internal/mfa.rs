use std::time::SystemTime;
use tokio::task::spawn_blocking;

use crate::prelude::*;

pub fn validate_2fa_token(token: &str) -> Result<(), Error> {
    match token.len() {
        6 => {
            if !token.chars().all(|c: char| c.is_ascii_digit()) {
                return Err(Error::TOTPRequired);
            }
        }
        13 => {
            // Taken from base32::Alphabet::Crockford
            if !token.bytes().all(|c: u8| b"0123456789ABCDEFGHJKMNPQRSTVWXYZ".contains(&c)) {
                return Err(Error::TOTPRequired);
            }
        }
        _ => return Err(Error::TOTPRequired),
    }

    Ok(())
}

enum MfaAttempt {
    Token(u32),
    Backup(u64),
}

impl MfaAttempt {
    pub fn parse(token: &str) -> Result<Self, Error> {
        match token.len() {
            // 6-digit TOTP code
            6 => match token.parse() {
                Ok(token) => Ok(MfaAttempt::Token(token)),
                Err(_) => Err(Error::InvalidCredentials),
            },
            // 13-character backup code
            13 => match base32::decode(base32::Alphabet::Crockford, token) {
                Some(token_bytes) if token_bytes.len() == 8 => {
                    let mut token = [0; 8];
                    token.copy_from_slice(&token_bytes);
                    Ok(MfaAttempt::Backup(u64::from_le_bytes(token)))
                }
                _ => Err(Error::InvalidCredentials),
            },
            _ => Err(Error::InvalidCredentials),
        }
    }
}

pub enum ProvidedMfa<'a> {
    Encrypted(&'a [u8]),
    Plain(&'a mfa_totp::MFA),
}

pub async fn process_2fa<'a>(
    state: &ServerState,
    user_id: UserId,
    mfa: ProvidedMfa<'a>,
    password: &str,
    token: &str,
) -> Result<bool, Error> {
    use crate::util::encrypt::nonce_from_user_id;
    use mfa_totp::{totp::TOTP6, MFA};
    use rand::Rng;

    // User's cannot do multiple 2fa requests at once
    let _user_id_guard = state.id_lock.lock(user_id).await;

    let token = MfaAttempt::parse(token)?;

    let mfa_key = state.config().local.keys.mfa_key;

    let nonce = nonce_from_user_id(user_id);

    let mfa = match mfa {
        ProvidedMfa::Plain(mfa) => *mfa,
        ProvidedMfa::Encrypted(encrypted_mfa) => {
            // SAFETY: These are only used within the following spawn_blocking block
            let password: &'static str = unsafe { std::mem::transmute(password) };
            let encrypted_mfa: &'static [u8] = unsafe { std::mem::transmute(encrypted_mfa) };

            let _permit = state.mem_semaphore.acquire_many(MFA::MEM_COST).await?;

            match spawn_blocking(move || MFA::decrypt(&mfa_key, &nonce, password, encrypted_mfa)).await? {
                Ok(mfa) => mfa,
                Err(_) => return Err(Error::InternalErrorStatic("Decrypt Error")),
            }
        }
    };

    match token {
        MfaAttempt::Token(token) => {
            let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();

            // Get the last used timestamp for this user's TOTP, or default to 0
            //
            // NOTE: Without the `id_lock _guard`, this might introduce security issues given `peek_with` is not linearizable,
            //  however, with the lock, this will never happen.
            let mut last = state.mfa_last.peek_with(&user_id, |_, last| *last).unwrap_or_default();

            if !TOTP6::new(&mfa.key).check(token, now, &mut last) {
                return Err(Error::InvalidCredentials);
            }

            use scc::hash_index::Entry;

            match state.mfa_last.entry_async(user_id).await {
                Entry::Occupied(entry) => entry.update(last),
                Entry::Vacant(entry) => {
                    entry.insert_entry(last);
                }
            }
        }
        MfaAttempt::Backup(code) => {
            let mut mfa = mfa; // make mutable

            let Some(idx) = mfa.backups.iter().position(|backup| code == *backup) else {
                return Err(Error::InvalidCredentials);
            };

            log::debug!("MFA Backup token used, saving new backup to database");

            // set old backup code to random value to prevent reuse
            mfa.backups[idx] = util::rng::crypto_thread_rng().gen();

            // SAFETY: This is only used within the following spawn_blocking block
            let password: &'static str = unsafe { std::mem::transmute(password) };

            let _permit = state.mem_semaphore.acquire_many(MFA::MEM_COST).await?;

            let Ok(encrypted_mfa) = spawn_blocking(move || mfa.encrypt(&mfa_key, &nonce, password)).await? else {
                return Err(Error::InternalErrorStatic("Encrypt error"));
            };

            drop(_permit);

            #[rustfmt::skip]
            state.db.write.get().await?.execute2(schema::sql! {
                UPDATE Users SET (Mfa) = (#{&encrypted_mfa as Users::Mfa})
                 WHERE Users.Id = #{&user_id as Users::Id}
            })
            .await?;
        }
    }

    drop(_user_id_guard);

    Ok(true)
}
