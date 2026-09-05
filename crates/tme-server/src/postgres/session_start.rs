use super::*;

impl PostgresState {
    pub async fn open(
        database_url: &str,
        bootstrap: PostgresBootstrap,
    ) -> Result<Arc<Self>, String> {
        Self::open_with_credentials(database_url, database_url, bootstrap).await
    }

    pub async fn open_with_credentials(
        database_url: &str,
        auth_database_url: &str,
        bootstrap: PostgresBootstrap,
    ) -> Result<Arc<Self>, String> {
        validate_bootstrap(&bootstrap)?;
        let pool = runtime_pool(database_url).await?;
        let auth_pool = auth_pool(auth_database_url).await?;
        migrations::verify(&pool).await?;
        crate::operator::verify_cluster_identity(&pool).await?;
        let store = Arc::new(PostgresStore::new(pool));
        let auth = AuthService::new().await?;

        let (facet_id, key, engine, revision, sequence) =
            recover_or_initialize(&store, bootstrap).await?;
        store.verify_player_kill_marks().await?;
        store.reconcile_all_player_kill_marks().await?;
        store.verify_player_kill_marks().await?;
        let ready = Arc::new(GameplayReadiness::new());
        let coordinator = Arc::new(Coordinator::new(store.clone()));
        let (handle, startup) = FacetHandle::spawn_persisted(
            facet_id,
            engine,
            revision,
            sequence,
            store.clone(),
            ready.clone(),
            coordinator.clone(),
        );
        startup
            .await
            .map_err(|_| "the persisted world failed before startup acknowledgement".to_string())?;
        let state = Arc::new(Self {
            store,
            auth_pool,
            world: Arc::new(RegisteredFacet {
                facet_id,
                handle,
                key,
            }),
            auth,
            coordinator,
            ready,
            next_transfer_epoch: Arc::new(AtomicU64::new(1)),
            live: Arc::new(Mutex::new(LiveState::default())),
            login_limits: Arc::new(Mutex::new(LoginLimits::default())),
            required_tasks: Arc::new(RequiredTaskLifecycle::default()),
        });
        state.reconcile_expired_sessions().await?;
        spawn_player_kill_mark_reconciler(&state)?;
        state.ready.seal_ready()?;
        Ok(state)
    }

    pub fn gameplay_ready(&self) -> bool {
        self.ready.is_ready()
    }

    pub(super) async fn commit_gameplay_transaction(
        &self,
        transaction: Transaction<'_, Postgres>,
    ) -> Result<(), sqlx::Error> {
        let result = transaction.commit().await;
        if result.is_err() {
            self.ready.fail();
        }
        result
    }

    pub(crate) fn coordinator(&self) -> Arc<Coordinator> {
        self.coordinator.clone()
    }

    pub fn facet_id_for_key(&self, key: &str) -> Option<wire::FacetId> {
        (self.world.key == key).then_some(self.world.facet_id)
    }

    pub(crate) fn maximum_mailbox_depth(&self) -> usize {
        self.world.handle.mailbox_depth()
    }

    pub(crate) async fn restore_fence_epoch(&self) -> Result<u64, String> {
        let value: i64 =
            sqlx::query_scalar("SELECT restore_fence_epoch FROM tme.store_state WHERE singleton")
                .fetch_one(self.store.pool())
                .await
                .map_err(|error| error.to_string())?;
        checked_u64(value)
    }

    pub async fn login(
        &self,
        source: IpAddr,
        request: wire::LoginRequestV1,
    ) -> Result<LoginSuccess, LoginError> {
        if !self.gameplay_ready() {
            return Err(LoginError::Unavailable);
        }
        {
            let mut limits = self
                .login_limits
                .lock()
                .map_err(|_| LoginError::Unavailable)?;
            if !limits.allow_source(source) {
                return Err(LoginError::RateLimited);
            }
        }
        let row = sqlx::query(
            "SELECT a.account_id, c.password_phc FROM tme.accounts a \
             JOIN tme.account_credentials c USING (account_id) \
             WHERE a.username=$1 AND a.status='active'",
        )
        .bind(request.username.as_str())
        .fetch_optional(&self.auth_pool)
        .await
        .map_err(|_| LoginError::Unavailable)?;
        let (account_id, phc) = match row {
            Some(row) => {
                let id: Uuid = row
                    .try_get("account_id")
                    .map_err(|_| LoginError::Unavailable)?;
                let account_id = wire::AccountId::new(id).map_err(|_| LoginError::Unavailable)?;
                let mut limits = self
                    .login_limits
                    .lock()
                    .map_err(|_| LoginError::Unavailable)?;
                if !limits.allow_account(account_id) {
                    return Err(LoginError::RateLimited);
                }
                let phc = row
                    .try_get("password_phc")
                    .map_err(|_| LoginError::Unavailable)?;
                (Some(account_id), phc)
            }
            None => (None, self.auth.dummy_phc.as_ref().clone()),
        };
        let verification = self
            .auth
            .verify(request.password.expose_for_verification(), phc)
            .await
            .map_err(|_| LoginError::Unavailable)?;
        let Some(account_id) = account_id.filter(|_| verification.verified) else {
            return Err(LoginError::InvalidCredentials);
        };
        let replacement = if verification.needs_rehash {
            Some(
                self.auth
                    .hash(request.password.expose_for_verification())
                    .await
                    .map_err(|_| LoginError::Unavailable)?,
            )
        } else {
            None
        };
        let session_token = random_secret().map_err(|_| LoginError::Unavailable)?;
        let csrf = random_csrf().map_err(|_| LoginError::Unavailable)?;
        let session_id =
            wire::SessionId::new(Uuid::now_v7()).map_err(|_| LoginError::Unavailable)?;
        let mut tx = serializable(&self.auth_pool)
            .await
            .map_err(|_| LoginError::Unavailable)?;
        let count: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM tme.sessions WHERE account_id=$1 AND revoked_at IS NULL \
             AND idle_expires_at > statement_timestamp() AND absolute_expires_at > statement_timestamp()",
        )
        .bind(account_id.as_uuid())
        .fetch_one(&mut *tx)
        .await
        .map_err(|_| LoginError::Unavailable)?;
        if count >= MAX_SESSIONS_PER_ACCOUNT as i64 {
            return Err(LoginError::Unavailable);
        }
        if let Some(replacement) = replacement {
            sqlx::query(
                "UPDATE tme.account_credentials SET password_phc=$2, \
                 credential_updated_at=statement_timestamp() WHERE account_id=$1",
            )
            .bind(account_id.as_uuid())
            .bind(replacement)
            .execute(&mut *tx)
            .await
            .map_err(|_| LoginError::Unavailable)?;
        }
        sqlx::query(
            "INSERT INTO tme.sessions \
             (session_id,account_id,token_digest,csrf_digest,idle_expires_at,absolute_expires_at) \
             VALUES ($1,$2,$3,$4,statement_timestamp()+make_interval(secs=>$5), \
                     statement_timestamp()+make_interval(secs=>$6))",
        )
        .bind(session_id.as_uuid())
        .bind(account_id.as_uuid())
        .bind(digest(session_token.expose()).as_slice())
        .bind(digest(csrf.expose_for_validation()).as_slice())
        .bind(checked_i64(SESSION_IDLE.as_secs()).map_err(|_| LoginError::Unavailable)?)
        .bind(checked_i64(SESSION_ABSOLUTE.as_secs()).map_err(|_| LoginError::Unavailable)?)
        .execute(&mut *tx)
        .await
        .map_err(|_| LoginError::Unavailable)?;
        audit(
            &mut tx,
            AuditEvent {
                account_id: Some(account_id.as_uuid()),
                session_id: Some(session_id.as_uuid()),
                character_id: None,
                command_id: None,
                actor: "runtime",
                action: "login",
                result: "success",
            },
        )
        .await
        .map_err(|_| LoginError::Unavailable)?;
        tx.commit().await.map_err(|_| LoginError::Unavailable)?;
        if let Ok(mut limits) = self.login_limits.lock() {
            limits.refund_source(source);
            limits.clear_account(account_id);
        }
        let bootstrap = self
            .bootstrap_for(session_id, account_id, csrf, None)
            .await
            .map_err(|_| LoginError::Unavailable)?;
        Ok(LoginSuccess {
            session_token,
            bootstrap,
        })
    }

    pub async fn session_bootstrap(
        &self,
        session_token: &str,
    ) -> Result<wire::SessionBootstrapV1, SessionError> {
        let csrf = random_csrf().map_err(|_| SessionError::Unavailable)?;
        let mut tx = serializable(self.store.pool())
            .await
            .map_err(|_| SessionError::Unavailable)?;
        let session = active_session(&mut tx, session_token, true)
            .await?
            .ok_or(SessionError::AuthenticationRequired)?;
        sqlx::query("UPDATE tme.sessions SET csrf_digest=$2 WHERE session_id=$1")
            .bind(session.session_id.as_uuid())
            .bind(digest(csrf.expose_for_validation()).as_slice())
            .execute(&mut *tx)
            .await
            .map_err(|_| SessionError::Unavailable)?;
        tx.commit().await.map_err(|_| SessionError::Unavailable)?;
        self.bootstrap_for(
            session.session_id,
            session.account_id,
            csrf,
            session.selected_character_id,
        )
        .await
    }
}
