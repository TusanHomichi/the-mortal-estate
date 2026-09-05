// Private process evidence for the EV certification target.
fn unused_loopback_addresses() -> (SocketAddr, SocketAddr) {
    let public = TcpListener::bind("127.0.0.1:0").unwrap();
    let operations = TcpListener::bind("127.0.0.1:0").unwrap();
    let addresses = (
        public.local_addr().unwrap(),
        operations.local_addr().unwrap(),
    );
    assert_ne!(addresses.0, addresses.1);
    addresses
}

fn spawn_server_child(
    manifest: &std::path::Path,
    credentials: &std::path::Path,
    public_address: SocketAddr,
    operations_address: SocketAddr,
    host: &str,
    origin: &str,
) -> ChildGuard {
    ChildGuard(
        Command::new(env!("CARGO_BIN_EXE_tme-server"))
            .arg("serve")
            .env_clear()
            .env("CREDENTIALS_DIRECTORY", credentials)
            .env("TME_PUBLIC_LISTEN", public_address.to_string())
            .env("TME_OPS_LISTEN", operations_address.to_string())
            .env("TME_PUBLIC_HOST", host)
            .env("TME_PUBLIC_ORIGIN", origin)
            .env("TME_BOOTSTRAP_MANIFEST", manifest)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    )
}

fn terminate_server_child(child: &mut ChildGuard) {
    let pid = child.0.id();
    let status = Command::new("kill")
        .arg("-TERM")
        .arg(pid.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap();
    assert!(status.success(), "failed to signal server child {pid}");
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        if let Some(status) = child.0.try_wait().unwrap() {
            assert!(status.success(), "server child {pid} failed during drain");
            return;
        }
        assert!(
            Instant::now() < deadline,
            "server child {pid} did not drain before its deadline"
        );
        std::thread::sleep(Duration::from_millis(25));
    }
}

fn crash_server_child(child: &mut ChildGuard) {
    let pid = child.0.id();
    child
        .0
        .kill()
        .unwrap_or_else(|error| panic!("failed to kill server child {pid}: {error}"));
    let status = child
        .0
        .wait()
        .unwrap_or_else(|error| panic!("failed to reap killed server child {pid}: {error}"));
    assert!(
        !status.success(),
        "killed server child {pid} exited successfully"
    );
}

async fn wait_for_child(
    address: SocketAddr,
    operations_address: SocketAddr,
    child: &mut ChildGuard,
) {
    let pid = child.0.id();
    tokio::task::spawn_blocking(move || {
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            if TcpStream::connect_timeout(&address, Duration::from_millis(100)).is_ok()
                && TcpStream::connect_timeout(&operations_address, Duration::from_millis(100))
                    .is_ok()
            {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "server child {pid} did not bind before its deadline"
            );
            std::thread::sleep(Duration::from_millis(50));
        }
    })
    .await
    .unwrap();
    assert!(
        child.0.try_wait().unwrap().is_none(),
        "server child exited early"
    );
    let mut watchdog = WallClockWatchdog::start(Duration::from_secs(20));
    let ready = async {
        loop {
            let (status, body) = support::operations_status(operations_address).await;
            if status == 200 && body["gameplay_ready"] == serde_json::Value::Bool(true) {
                return;
            }
            wall_clock_delay(Duration::from_millis(25)).await;
        }
    };
    tokio::select! {
        () = ready => {}
        expired = &mut watchdog.expired => {
            expired.expect("child readiness watchdog sender");
            panic!("server child {pid} did not become ready before its deadline");
        }
    }
}

async fn assert_runner_identity(pool: &sqlx::PgPool, database: &str, sentinel: &str, role: &str) {
    let row = sqlx::query(
        "SELECT current_database() AS database, current_user AS role, \
         shobj_description(d.oid,'pg_database') AS comment \
         FROM pg_database d WHERE d.datname=current_database()",
    )
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(database, row.get::<String, _>("database"));
    assert_eq!(role, row.get::<String, _>("role"));
    assert_eq!(
        format!("tme_ev:{sentinel}"),
        row.get::<String, _>("comment")
    );
    assert!(
        sqlx::query_scalar::<_, bool>(
            "SELECT database_oid=(SELECT oid::text FROM pg_database WHERE datname=current_database()) \
             FROM tme.store_state WHERE singleton"
        )
        .fetch_one(pool)
        .await
        .unwrap()
    );
}

async fn insert_account(pool: &sqlx::PgPool, fixture: &CharacterFixture, salt_byte: u8) {
    let params = Params::new(65_536, 3, 4, Some(32)).unwrap();
    let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let salt = SaltString::encode_b64(&[salt_byte; 16]).unwrap();
    let phc = argon
        .hash_password(support::PASSWORD.as_bytes(), &salt)
        .unwrap()
        .to_string();
    sqlx::query("INSERT INTO tme.accounts(account_id,username,display_name) VALUES($1,$2,$3)")
        .bind(fixture.account_id.as_uuid())
        .bind(&fixture.username)
        .bind(format!("EV {}", fixture.username))
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO tme.account_credentials(account_id,password_phc) VALUES($1,$2)")
        .bind(fixture.account_id.as_uuid())
        .bind(phc)
        .execute(pool)
        .await
        .unwrap();
}

/// Grows the scenario's single seeded player into `count` distinct actors, all
/// inside the one world this process hosts.
fn engine_with_characters(count: usize) -> (tme_rules::Engine, Vec<tme_rules::ActorId>) {
    let prefix = "ev";
    let mut engine = scenario_engine();
    let original = engine.world().actors[0].clone();
    let original_character = original.character_id.clone().unwrap();
    let preferences = engine
        .world()
        .communication_preferences
        .get(&original_character)
        .cloned()
        .unwrap_or_default();
    let presence = engine
        .world()
        .character_presence
        .get(&original_character)
        .copied()
        .unwrap();
    let quest = engine
        .world()
        .quest_states
        .get(&original_character)
        .cloned();
    let mut actors = vec![original.id.clone()];
    for index in 1..count {
        let temporary_character =
            tme_rules::CharacterId::new(format!("prototype:ev:{prefix}:{index}"));
        let mut actor = original.clone();
        actor.id = tme_rules::ActorId::new(format!("{prefix}_{index}"));
        actor.name = format!("EV {prefix} {index}");
        actor.character_id = Some(temporary_character.clone());
        actor.timing.tie_break_order += u64::try_from(index * 100).unwrap();
        actor.carried.items.clear();
        actor.carried.gold = Default::default();
        actors.push(actor.id.clone());
        engine.world_mut().actors.push(actor);
        engine
            .world_mut()
            .communication_preferences
            .insert(temporary_character.clone(), preferences.clone());
        engine
            .world_mut()
            .character_presence
            .insert(temporary_character.clone(), presence);
        if let Some(quest) = &quest {
            engine
                .world_mut()
                .quest_states
                .insert(temporary_character, quest.clone());
        }
    }
    engine.export_checkpoint().unwrap();
    (engine, actors)
}

fn scenario_engine() -> tme_rules::Engine {
    let mut scenario = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    scenario.extend([
        "..",
        "..",
        "content",
        "test-corpus",
        "world_topology_gallery.json",
    ]);
    tme_sim::load_engine_from_scenario(&scenario, Some(7)).unwrap()
}
