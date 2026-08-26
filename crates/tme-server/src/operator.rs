use std::collections::BTreeSet;
use std::io::{self, IsTerminal, Read};
use std::path::Path;

use sha2::{Digest, Sha256};
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Row};
use tme_protocol as wire;
use tme_rules::FacetCheckpointV4;
use uuid::Uuid;

use crate::auth::{AuthService, normalize, validate_enrollment};
use crate::postgres::{MAX_BLOCKLIST_ENTRIES, MIN_BLOCKLIST_ENTRIES};
use crate::store::migrations;
use crate::store::{AuditEvent, audit, serializable};

pub async fn run(arguments: &[String]) -> Result<bool, String> {
    match arguments {
        [command] if command == "migrate" => {
            let pool = operator_pool().await?;
            migrations::migrate(&pool).await?;
            Ok(true)
        }
        [store, command] if store == "store" && command == "verify" => {
            let pool = operator_pool().await?;
            verify_store(&pool).await?;
            Ok(true)
        }
        [store, command, confirmation]
            if store == "store"
                && command == "restore-fence"
                && confirmation == "--confirm-restored-database" =>
        {
            let pool = operator_pool().await?;
            restore_fence(&pool).await?;
            Ok(true)
        }
        [bootstrap, command, path] if bootstrap == "bootstrap" && command == "verify" => {
            let bootstrap = crate::production::load_bootstrap(Path::new(path))?;
            crate::postgres::validate_bootstrap(&bootstrap)?;
            println!("bootstrap verify: ok");
            Ok(true)
        }
        [account, command, tail @ ..] if account == "account" && command == "create" => {
            create_account(tail).await?;
            Ok(true)
        }
        [account, command, tail @ ..]
            if account == "account" && command == "set-password" =>
        {
            set_password(tail).await?;
            Ok(true)
        }
        [] => Ok(false),
        _ => Err("usage: tme-server [migrate | bootstrap verify <path> | account create --username <name> --display-name <name> --compromised-passwords <path> | account set-password --username <name> --compromised-passwords <path> | store verify | store restore-fence --confirm-restored-database]".to_string()),
    }
}

async fn operator_pool() -> Result<PgPool, String> {
    let database_url = match std::env::var("DATABASE_URL") {
        Ok(value) if !value.is_empty() => value,
        _ => crate::production::read_systemd_credential("migrator-database-url")
            .map_err(|_| "an offline database credential is required".to_string())?,
    };
    PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(std::time::Duration::from_secs(5))
        .connect(&database_url)
        .await
        .map_err(|error| error.to_string())
}

async fn create_account(arguments: &[String]) -> Result<(), String> {
    let username = flag(arguments, "--username")?;
    let display_name = flag(arguments, "--display-name")?;
    let blocklist_path = flag(arguments, "--compromised-passwords")?;
    if arguments.len() != 6 {
        return Err("account create accepts exactly the three required flags".to_string());
    }
    let username = wire::Username::new(username).map_err(|error| error.to_string())?;
    let display_name = wire::DisplayName::new(display_name).map_err(|error| error.to_string())?;
    let blocklist = load_blocklist(Path::new(&blocklist_path))?;
    let (password, confirmation) = read_password_pair()?;
    if password != confirmation {
        return Err("password confirmation differs".to_string());
    }
    let password = wire::Password::new(password).map_err(|error| error.to_string())?;
    validate_enrollment(&username, &display_name, &password, &blocklist)?;
    let auth = AuthService::new().await?;
    let phc = auth.hash(password.expose_for_verification()).await?;
    let account_id = wire::AccountId::new(Uuid::now_v7()).map_err(|error| error.to_string())?;
    let pool = operator_pool().await?;
    migrations::verify(&pool).await?;
    let mut tx = serializable(&pool).await?;
    sqlx::query("INSERT INTO tme.accounts (account_id,username,display_name) VALUES ($1,$2,$3)")
        .bind(account_id.as_uuid())
        .bind(username.as_str())
        .bind(display_name.as_str())
        .execute(&mut *tx)
        .await
        .map_err(|error| format!("account insert failed: {error}"))?;
    sqlx::query("INSERT INTO tme.account_credentials (account_id,password_phc) VALUES ($1,$2)")
        .bind(account_id.as_uuid())
        .bind(phc)
        .execute(&mut *tx)
        .await
        .map_err(|error| format!("credential insert failed: {error}"))?;
    sqlx::query(
        "INSERT INTO tme.audit_events (account_id,actor,action,result,correlation_id) \
         VALUES ($1,'operator','account_create','success',$2)",
    )
    .bind(account_id.as_uuid())
    .bind(Uuid::now_v7())
    .execute(&mut *tx)
    .await
    .map_err(|error| error.to_string())?;
    tx.commit().await.map_err(|error| error.to_string())?;
    println!("{account_id}");
    Ok(())
}

struct PasswordChangeAccount {
    account_id: Uuid,
    display_name: String,
}

trait PasswordChangeStore {
    async fn account_for_username(
        &self,
        username: &wire::Username,
    ) -> Result<Option<PasswordChangeAccount>, String>;

    async fn write_password(
        &self,
        username: &wire::Username,
        account_id: Uuid,
        password_phc: String,
    ) -> Result<(), String>;
}

struct PostgresPasswordChangeStore<'a> {
    pool: &'a PgPool,
}

impl PasswordChangeStore for PostgresPasswordChangeStore<'_> {
    async fn account_for_username(
        &self,
        username: &wire::Username,
    ) -> Result<Option<PasswordChangeAccount>, String> {
        let row = sqlx::query("SELECT account_id,display_name FROM tme.accounts WHERE username=$1")
            .bind(username.as_str())
            .fetch_optional(self.pool)
            .await
            .map_err(|error| format!("account lookup failed: {error}"))?;
        row.map(|row| {
            Ok(PasswordChangeAccount {
                account_id: row
                    .try_get("account_id")
                    .map_err(|error| error.to_string())?,
                display_name: row
                    .try_get("display_name")
                    .map_err(|error| error.to_string())?,
            })
        })
        .transpose()
    }

    async fn write_password(
        &self,
        username: &wire::Username,
        account_id: Uuid,
        password_phc: String,
    ) -> Result<(), String> {
        let mut tx = serializable(self.pool).await?;
        let result = sqlx::query(
            "UPDATE tme.account_credentials AS c \
             SET password_phc=$3,credential_updated_at=statement_timestamp() \
             FROM tme.accounts AS a \
             WHERE c.account_id=a.account_id AND a.account_id=$1 AND a.username=$2",
        )
        .bind(account_id)
        .bind(username.as_str())
        .bind(password_phc)
        .execute(&mut *tx)
        .await
        .map_err(|error| format!("credential update failed: {error}"))?;
        if result.rows_affected() != 1 {
            return Err("account username does not exist".to_string());
        }
        sqlx::query(
            "INSERT INTO tme.audit_events (account_id,actor,action,result,correlation_id) \
             VALUES ($1,'operator','account_set_password','success',$2)",
        )
        .bind(account_id)
        .bind(Uuid::now_v7())
        .execute(&mut *tx)
        .await
        .map_err(|error| error.to_string())?;
        tx.commit().await.map_err(|error| error.to_string())
    }
}

async fn set_password(arguments: &[String]) -> Result<(), String> {
    let (username, blocklist_path) = set_password_flags(arguments)?;
    let username = wire::Username::new(username).map_err(|error| error.to_string())?;
    let blocklist = load_blocklist(Path::new(&blocklist_path))?;
    let (password, confirmation) = read_password_pair()?;
    if password != confirmation {
        return Err("password confirmation differs".to_string());
    }
    let password = wire::Password::new(password).map_err(|error| error.to_string())?;
    let pool = operator_pool().await?;
    migrations::verify(&pool).await?;
    set_password_with_store(
        &PostgresPasswordChangeStore { pool: &pool },
        username,
        password,
        &blocklist,
    )
    .await
}

fn set_password_flags(arguments: &[String]) -> Result<(String, String), String> {
    let username = flag(arguments, "--username")?;
    let blocklist_path = flag(arguments, "--compromised-passwords")?;
    if arguments.len() != 4 {
        return Err("account set-password accepts exactly the two required flags".to_string());
    }
    Ok((username, blocklist_path))
}

async fn set_password_with_store<S: PasswordChangeStore>(
    store: &S,
    username: wire::Username,
    password: wire::Password,
    blocklist: &BTreeSet<String>,
) -> Result<(), String> {
    let account = store
        .account_for_username(&username)
        .await?
        .ok_or_else(|| "account username does not exist".to_string())?;
    let display_name =
        wire::DisplayName::new(account.display_name).map_err(|error| error.to_string())?;
    validate_enrollment(&username, &display_name, &password, blocklist)?;
    let auth = AuthService::new().await?;
    let phc = auth.hash(password.expose_for_verification()).await?;
    store
        .write_password(&username, account.account_id, phc)
        .await
}

fn flag(arguments: &[String], name: &str) -> Result<String, String> {
    let indexes = arguments
        .iter()
        .enumerate()
        .filter_map(|(index, value)| (value == name).then_some(index))
        .collect::<Vec<_>>();
    if indexes.len() != 1 || indexes[0] + 1 >= arguments.len() {
        return Err(format!("{name} is required exactly once"));
    }
    let value = &arguments[indexes[0] + 1];
    if value.starts_with("--") {
        return Err(format!("{name} requires a value"));
    }
    Ok(value.clone())
}

fn load_blocklist(path: &Path) -> Result<BTreeSet<String>, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("compromised-password file could not be read: {error}"))?;
    if text.contains('\0') {
        return Err("compromised-password file contains NUL".to_string());
    }
    let values = text.lines().map(normalize).collect::<BTreeSet<_>>();
    if values.iter().any(String::is_empty)
        || !(MIN_BLOCKLIST_ENTRIES..=MAX_BLOCKLIST_ENTRIES).contains(&values.len())
    {
        return Err(format!(
            "compromised-password file must contain {MIN_BLOCKLIST_ENTRIES}-{MAX_BLOCKLIST_ENTRIES} unique nonempty entries"
        ));
    }
    Ok(values)
}

fn read_password_pair() -> Result<(String, String), String> {
    if io::stdin().is_terminal() {
        let password =
            rpassword::prompt_password("Password: ").map_err(|error| error.to_string())?;
        let confirmation =
            rpassword::prompt_password("Confirm password: ").map_err(|error| error.to_string())?;
        return Ok((password, confirmation));
    }
    let mut input = String::new();
    io::stdin()
        .take((wire::MAX_CONTROL_INPUT_BYTES * 2 + 3) as u64)
        .read_to_string(&mut input)
        .map_err(|error| error.to_string())?;
    let values = input.lines().collect::<Vec<_>>();
    if values.len() != 2 || input.as_bytes().last().copied() != Some(b'\n') {
        return Err(
            "nonterminal password input must contain exactly two newline-delimited values"
                .to_string(),
        );
    }
    Ok((values[0].to_string(), values[1].to_string()))
}

pub async fn verify_store(pool: &PgPool) -> Result<(), String> {
    migrations::verify(pool).await?;
    verify_cluster_identity(pool).await?;
    crate::store::PostgresStore::new(pool.clone())
        .verify_player_kill_marks()
        .await?;
    let rows = sqlx::query("SELECT checkpoint_bytes,checkpoint_sha256 FROM tme.facets")
        .fetch_all(pool)
        .await
        .map_err(|error| error.to_string())?;
    for row in rows {
        let bytes: Vec<u8> = row
            .try_get("checkpoint_bytes")
            .map_err(|error| error.to_string())?;
        let expected: Vec<u8> = row
            .try_get("checkpoint_sha256")
            .map_err(|error| error.to_string())?;
        if Sha256::digest(&bytes).as_slice() != expected.as_slice() {
            return Err("a durable checkpoint hash differs from its bytes".to_string());
        }
        FacetCheckpointV4::from_bytes(bytes).map_err(|error| error.to_string())?;
    }
    let receipts =
        sqlx::query("SELECT request_digest,disposition,outcome_bytes FROM tme.command_receipts")
            .fetch_all(pool)
            .await
            .map_err(|error| error.to_string())?;
    for row in receipts {
        crate::store::receipt::StoredReceipt::decode(row)?;
    }
    Ok(())
}

pub(crate) async fn verify_cluster_identity(pool: &PgPool) -> Result<(), String> {
    let current_cluster: String =
        sqlx::query_scalar("SELECT system_identifier::text FROM pg_control_system()")
            .fetch_one(pool)
            .await
            .map_err(|error| error.to_string())?;
    let current_database: String =
        sqlx::query_scalar("SELECT oid::text FROM pg_database WHERE datname=current_database()")
            .fetch_one(pool)
            .await
            .map_err(|error| error.to_string())?;
    let row = sqlx::query(
        "SELECT cluster_system_identifier,database_oid FROM tme.store_state WHERE singleton",
    )
    .fetch_one(pool)
    .await
    .map_err(|error| error.to_string())?;
    let stored_cluster: String = row
        .try_get("cluster_system_identifier")
        .map_err(|error| error.to_string())?;
    let stored_database: String = row
        .try_get("database_oid")
        .map_err(|error| error.to_string())?;
    if current_cluster != stored_cluster || current_database != stored_database {
        return Err("database restore identity differs and requires restore-fence".to_string());
    }
    Ok(())
}

async fn restore_fence(pool: &PgPool) -> Result<(), String> {
    migrations::verify(pool).await?;
    let current_cluster: String =
        sqlx::query_scalar("SELECT system_identifier::text FROM pg_control_system()")
            .fetch_one(pool)
            .await
            .map_err(|error| error.to_string())?;
    let mut tx = serializable(pool).await?;
    let current_database: String =
        sqlx::query_scalar("SELECT oid::text FROM pg_database WHERE datname=current_database()")
            .fetch_one(&mut *tx)
            .await
            .map_err(|error| error.to_string())?;
    sqlx::query("SELECT singleton FROM tme.store_state WHERE singleton FOR UPDATE")
        .fetch_one(&mut *tx)
        .await
        .map_err(|error| error.to_string())?;
    sqlx::query("UPDATE tme.sessions SET revoked_at=COALESCE(revoked_at,statement_timestamp())")
        .execute(&mut *tx)
        .await
        .map_err(|error| error.to_string())?;
    sqlx::query("DELETE FROM tme.socket_tickets WHERE consumed_at IS NULL")
        .execute(&mut *tx)
        .await
        .map_err(|error| error.to_string())?;
    sqlx::query("UPDATE tme.characters SET control_epoch=control_epoch+1")
        .execute(&mut *tx)
        .await
        .map_err(|error| error.to_string())?;
    sqlx::query("UPDATE tme.store_state SET cluster_system_identifier=$1,database_oid=$2,last_restore_fenced_at=statement_timestamp(),restore_fence_epoch=restore_fence_epoch+1 WHERE singleton")
        .bind(current_cluster).bind(current_database).execute(&mut *tx).await.map_err(|error| error.to_string())?;
    audit(
        &mut tx,
        AuditEvent {
            account_id: None,
            session_id: None,
            character_id: None,
            command_id: None,
            actor: "recovery",
            action: "restore_fence",
            result: "success",
        },
    )
    .await?;
    tx.commit().await.map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct FakePasswordChangeStore {
        account: Option<PasswordChangeAccount>,
        writes: AtomicUsize,
    }

    impl PasswordChangeStore for FakePasswordChangeStore {
        async fn account_for_username(
            &self,
            _username: &wire::Username,
        ) -> Result<Option<PasswordChangeAccount>, String> {
            Ok(self.account.as_ref().map(|account| PasswordChangeAccount {
                account_id: account.account_id,
                display_name: account.display_name.clone(),
            }))
        }

        async fn write_password(
            &self,
            _username: &wire::Username,
            _account_id: Uuid,
            _password_phc: String,
        ) -> Result<(), String> {
            self.writes.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    #[test]
    fn account_flags_are_order_independent_and_exact() {
        let values = vec![
            "--display-name".to_string(),
            "Player".to_string(),
            "--username".to_string(),
            "player_1".to_string(),
            "--compromised-passwords".to_string(),
            "/tmp/list".to_string(),
        ];
        assert_eq!(flag(&values, "--username").unwrap(), "player_1");
        assert!(flag(&values, "--missing").is_err());
    }

    #[test]
    fn set_password_flags_are_order_independent_and_exact() {
        let values = vec![
            "--compromised-passwords".to_string(),
            "/tmp/list".to_string(),
            "--username".to_string(),
            "player_1".to_string(),
        ];
        assert_eq!(
            set_password_flags(&values).unwrap(),
            ("player_1".to_string(), "/tmp/list".to_string())
        );
        let mut unexpected = values.clone();
        unexpected.extend(["--display-name".to_string(), "Player".to_string()]);
        assert!(set_password_flags(&unexpected).is_err());
        let duplicate = vec![
            "--username".to_string(),
            "player_1".to_string(),
            "--username".to_string(),
            "player_2".to_string(),
        ];
        assert!(set_password_flags(&duplicate).is_err());
    }

    #[tokio::test]
    async fn set_password_unknown_username_fails_without_write() {
        let store = FakePasswordChangeStore {
            account: None,
            writes: AtomicUsize::new(0),
        };
        let error = set_password_with_store(
            &store,
            wire::Username::new("player_1").unwrap(),
            wire::Password::new("a valid new password").unwrap(),
            &BTreeSet::new(),
        )
        .await
        .unwrap_err();
        assert_eq!(error, "account username does not exist");
        assert_eq!(store.writes.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn set_password_rejects_invalid_enrollment_before_write() {
        let store = FakePasswordChangeStore {
            account: Some(PasswordChangeAccount {
                account_id: Uuid::now_v7(),
                display_name: "Player One".to_string(),
            }),
            writes: AtomicUsize::new(0),
        };
        let error = set_password_with_store(
            &store,
            wire::Username::new("player_1").unwrap(),
            wire::Password::new("too short").unwrap(),
            &BTreeSet::new(),
        )
        .await
        .unwrap_err();
        assert_eq!(error, "password must contain 15-128 Unicode scalars");
        assert_eq!(store.writes.load(Ordering::SeqCst), 0);
    }
}
