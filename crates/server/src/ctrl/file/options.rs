use models::{Snowflake, UserFlags};
use schema::SnowflakeExt;
use timestamp::Timestamp;

use crate::{ctrl::Error, web::auth::Authorization, ServerState};

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct FileOptions {
    pub quota_used: i64,
    pub quota_total: i64,
}

pub async fn file_options(state: ServerState, auth: Authorization) -> Result<FileOptions, Error> {
    let month_start = {
        let (year, month, _) = Timestamp::now_utc().date().to_calendar_date();
        Snowflake::at_date(time::Date::from_calendar_date(year, month, 1).unwrap())
    };

    let quota_total = if auth.flags.contains(UserFlags::PREMIUM) {
        state.config.monthly_premium_upload_quota
    } else {
        state.config.monthly_upload_quota
    };

    let db = state.db.read.get().await?;

    let row = db
        .query_one_cached_typed(
            || {
                use schema::*;
                use thorn::*;

                Query::select()
                    .from_table::<Files>()
                    .expr(Builtin::sum(Files::Size))
                    .and_where(Files::UserId.equals(Var::of(Users::Id)))
                    .and_where(Files::Id.greater_than_equal(Var::of(Files::Id)))
            },
            &[&auth.user_id, &month_start],
        )
        .await?;

    let quota_used: Option<i64> = row.try_get(0)?;

    Ok(FileOptions {
        quota_used: quota_used.unwrap_or(0),
        quota_total,
    })
}
