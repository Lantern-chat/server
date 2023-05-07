use schema::SnowflakeExt;
use sdk::models::{Snowflake, UserFlags};
use timestamp::Timestamp;

use crate::{Authorization, Error, ServerState};

use sdk::api::commands::file::FilesystemStatus;

pub async fn file_options(state: &ServerState, auth: Authorization) -> Result<FilesystemStatus, Error> {
    let month_start = {
        let (year, month, _) = Timestamp::now_utc().date().to_calendar_date();
        Snowflake::at_date(time::Date::from_calendar_date(year, month, 1).unwrap())
    };

    #[rustfmt::skip]
    let row = state.db.read.get().await?.query_one2(schema::sql! {
        SELECT
            SUM(Files.Size) AS @QuotaUsed
        FROM Files WHERE
            Files.UserId = #{&auth.user_id as Files::UserId}
        AND Files.Id    >= #{&month_start  as Files::Id}
    }).await?;

    let quota_used: Option<i64> = row.quota_used()?;

    let config = state.config();

    Ok(FilesystemStatus {
        quota_used: quota_used.unwrap_or(0),
        quota_total: if auth.flags.contains(UserFlags::PREMIUM) {
            config.upload.monthly_premium_upload_quota
        } else {
            config.upload.monthly_upload_quota
        },
    })
}
