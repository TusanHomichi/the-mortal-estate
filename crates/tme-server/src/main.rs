#[tokio::main]
async fn main() -> std::io::Result<()> {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    if arguments.as_slice() == ["serve"] {
        return serve_production().await;
    }
    match tme_server::operator::run(&arguments).await {
        Ok(true) => return Ok(()),
        Ok(false) => {}
        Err(error) => {
            eprintln!("{error}");
            return Err(std::io::Error::other("offline command failed"));
        }
    }
    tme_server::telemetry::init();
    let config = tme_server::ServerConfig::loopback_default();
    let listener = tokio::net::TcpListener::bind(config.listen).await?;
    let state = tme_server::AppState::disabled(config);
    tracing::info!(address = %listener.local_addr()?, gameplay_ready = false, "The Mortal Estate ES server listening without provisioned PostgreSQL authority");
    tme_server::runtime::serve(listener, state, async {
        let _ = tokio::signal::ctrl_c().await;
    })
    .await
}

async fn serve_production() -> std::io::Result<()> {
    tme_server::telemetry::init();
    let public_listen = required_address("TME_PUBLIC_LISTEN")?;
    let operations_listen = required_address("TME_OPS_LISTEN")?;
    if !operations_listen.ip().is_loopback() || operations_listen == public_listen {
        return Err(std::io::Error::other(
            "operations listener must be distinct and loopback-only",
        ));
    }
    let public_host = required_environment("TME_PUBLIC_HOST")?;
    let public_origin = required_environment("TME_PUBLIC_ORIGIN")?;
    let bootstrap_path = std::path::PathBuf::from(required_environment("TME_BOOTSTRAP_MANIFEST")?);
    let config = tme_server::ServerConfig::new(public_listen, public_host, public_origin)
        .map_err(std::io::Error::other)?;
    let bootstrap =
        tme_server::production::load_bootstrap(&bootstrap_path).map_err(std::io::Error::other)?;
    let database_url = tme_server::production::read_systemd_credential("database-url")
        .map_err(std::io::Error::other)?;
    let auth_database_url = tme_server::production::read_systemd_credential("auth-database-url")
        .map_err(std::io::Error::other)?;
    let backend = tme_server::PostgresState::open_with_credentials(
        &database_url,
        &auth_database_url,
        bootstrap,
    )
    .await
    .map_err(std::io::Error::other)?;
    let public_listener = tokio::net::TcpListener::bind(config.listen).await?;
    let operations_listener = tokio::net::TcpListener::bind(operations_listen).await?;
    let state = tme_server::AppState::postgres(config, backend);
    tracing::info!(
        public_address = %public_listener.local_addr()?,
        operations_address = %operations_listener.local_addr()?,
        gameplay_ready = true,
        "The Mortal Estate production runtime ready"
    );
    tme_server::runtime::serve_production(
        public_listener,
        operations_listener,
        state,
        shutdown_signal(),
    )
    .await
}

fn required_environment(name: &str) -> std::io::Result<String> {
    std::env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty() && value != "REQUIRED")
        .ok_or_else(|| std::io::Error::other(format!("{name} is required")))
}

fn required_address(name: &str) -> std::io::Result<std::net::SocketAddr> {
    required_environment(name)?
        .parse()
        .map_err(|_| std::io::Error::other(format!("{name} is not a socket address")))
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("SIGTERM handler");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {},
            _ = terminate.recv() => {},
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}
