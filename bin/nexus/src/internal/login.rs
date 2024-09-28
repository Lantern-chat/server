use std::{net::IpAddr, time::SystemTime};

use schema::auth::RawAuthToken;
use sdk::models::Session;

use crate::prelude::*;

pub async fn do_login(state: ServerState, addr: IpAddr, user_id: UserId) -> Result<Session, Error> {
    let now = SystemTime::now();

    let token = RawAuthToken::bearer(util::rng::crypto_thread_rng());
    let bytes = match token {
        RawAuthToken::Bearer(ref bytes) => &bytes[..],
        _ => unreachable!(),
    };

    let expires = now + state.config().shared.session_duration;

    let db = state.db.write.get().await?;

    db.execute2(schema::sql! {
        INSERT INTO Sessions (
            Token, UserId, Expires, Addr
        ) VALUES (
            #{&bytes    as Sessions::Token   },
            #{&user_id  as Sessions::UserId  },
            #{&expires  as Sessions::Expires },
            #{&addr     as Sessions::Addr    }
        )
    })
    .await?;

    Ok(Session {
        auth: token.into(),
        expires: expires.into(),
    })
}
