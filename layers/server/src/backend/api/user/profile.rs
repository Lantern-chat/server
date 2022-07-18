use futures::{future::Either, TryFutureExt};
use sdk::{api::commands::user::UpdateUserProfileBody, models::*};

use crate::{backend::util::encrypted_asset::encrypt_snowflake, Authorization, Error, ServerState};
