use sqlx::{PgPool, Row};

pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

pub async fn migrate(pool: &PgPool) -> Result<(), String> {
    verify_postgres_major(pool).await?;
    MIGRATOR.run(pool).await.map_err(|error| error.to_string())
}

pub async fn verify(pool: &PgPool) -> Result<(), String> {
    verify_postgres_major(pool).await?;
    let table_exists: bool =
        sqlx::query_scalar("SELECT to_regclass('_sqlx_migrations') IS NOT NULL")
            .fetch_one(pool)
            .await
            .map_err(|error| error.to_string())?;
    if !table_exists {
        return Err("migration history is missing".to_string());
    }
    let rows = sqlx::query(
        "SELECT version, description, success, checksum FROM _sqlx_migrations ORDER BY version",
    )
    .fetch_all(pool)
    .await
    .map_err(|error| error.to_string())?;
    let expected = MIGRATOR
        .iter()
        .filter(|migration| !migration.migration_type.is_down_migration())
        .collect::<Vec<_>>();
    if rows.len() != expected.len() {
        return Err("migration history count differs from the embedded set".to_string());
    }
    for (row, migration) in rows.into_iter().zip(expected) {
        let version: i64 = row.try_get("version").map_err(|error| error.to_string())?;
        let description: String = row
            .try_get("description")
            .map_err(|error| error.to_string())?;
        let success: bool = row.try_get("success").map_err(|error| error.to_string())?;
        let checksum: Vec<u8> = row.try_get("checksum").map_err(|error| error.to_string())?;
        if !success
            || version != migration.version
            || description != migration.description
            || checksum.as_slice() != migration.checksum.as_ref()
        {
            return Err(format!("migration history mismatch at version {version}"));
        }
    }
    Ok(())
}

async fn verify_postgres_major(pool: &PgPool) -> Result<(), String> {
    let version: String = sqlx::query_scalar("SHOW server_version_num")
        .fetch_one(pool)
        .await
        .map_err(|error| error.to_string())?;
    let version = version
        .parse::<i32>()
        .map_err(|_| "server_version_num is not an integer".to_string())?;
    if version / 10_000 != 18 {
        return Err(format!(
            "PostgreSQL major 18 is required; server_version_num={version}"
        ));
    }
    Ok(())
}
