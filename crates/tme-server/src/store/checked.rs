use sqlx::PgConnection;
use uuid::Uuid;

pub(crate) struct ReceiptRecord {
    pub request_digest: Vec<u8>,
    pub disposition: String,
    pub outcome_bytes: Option<Vec<u8>>,
    pub full_expired: bool,
}

pub(crate) async fn receipt(
    connection: &mut PgConnection,
    account_id: Uuid,
    command_id: Uuid,
) -> Result<Option<ReceiptRecord>, sqlx::Error> {
    sqlx::query_as!(
        ReceiptRecord,
        r#"
        SELECT request_digest AS "request_digest!",
               disposition AS "disposition!",
               outcome_bytes,
               (full_expires_at <= statement_timestamp()) AS "full_expired!"
        FROM tme.command_receipts
        WHERE account_id = $1 AND command_id = $2
        FOR UPDATE
        "#,
        account_id,
        command_id,
    )
    .fetch_optional(connection)
    .await
}
