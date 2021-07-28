use crate::{
    ctrl::Error,
    web::{auth::Authorization, routes::api::v1::file::post::Metadata},
    ServerState,
};

use schema::Snowflake;

pub struct UploadHead {
    pub offset: i32,
}

pub async fn head(state: ServerState, auth: Authorization, file_id: Snowflake) -> Result<UploadHead, Error> {
    let fetch_record = async {
        let db = state.db.read.get().await?;

        Ok::<_, Error>(())
    };

    let fetch_metadata = async {
        let file_lock = state.id_lock.get(file_id).await;
        let _guard = file_lock.lock().await;

        Ok::<_, Error>(())
    };

    let (record, metadata) = tokio::try_join!(fetch_record, fetch_metadata)?;

    unimplemented!()
}
