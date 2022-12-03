use server::backend::db::DatabasePools;

use server::Error;

pub async fn verify_schema(pool: &DatabasePools) -> Result<(), Error> {
    let db = pool.read.get().await?;

    for query in schema::verify::iterate_tables_str() {
        for row in db.query(&query, &[]).await? {
            let valid: Option<bool> = row.try_get(0)?;

            if Some(true) != valid {
                let column_name: &str = row.try_get(1)?;
                let table_name: &str = row.try_get(2)?;
                let schema_name: &str = row.try_get(3)?;
                let expected_udt: &str = row.try_get(4)?;
                let found_udt: Option<&str> = row.try_get(5)?;

                log::error!(
                    "Schema verification mismatch for {}.{}.{}, expected {}, found {}",
                    schema_name,
                    table_name,
                    column_name,
                    expected_udt,
                    found_udt.unwrap_or("NULL")
                );
            }
        }
    }

    Ok(())
}
