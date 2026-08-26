use std::future::Future;

use crate::config::DRAIN_TIMEOUT;
use crate::http::AppState;
use tokio::net::TcpListener;

pub async fn serve(
    listener: TcpListener,
    state: AppState,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> std::io::Result<()> {
    let router = crate::http::router(state.clone());
    let mut draining_receive = state.subscribe_draining();
    let drain_state = state.clone();
    tokio::spawn(async move {
        shutdown.await;
        drain_state.begin_draining();
    });
    let mut server_drain = draining_receive.clone();
    let server = axum::serve(
        listener,
        router.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(async move {
        wait_for_drain(&mut server_drain).await;
    });
    tokio::select! {
        result = server => result,
        _ = async move {
            wait_for_drain(&mut draining_receive).await;
            tokio::time::sleep(DRAIN_TIMEOUT).await;
        } => Ok(()),
    }
}

pub async fn serve_production(
    public_listener: TcpListener,
    operations_listener: TcpListener,
    state: AppState,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> std::io::Result<()> {
    let public = crate::http::router(state.clone());
    let operations = crate::operations::router(state.clone());
    let drain_state = state.clone();
    tokio::spawn(async move {
        shutdown.await;
        drain_state.begin_draining();
    });
    let mut public_drain = state.subscribe_draining();
    let mut operations_drain = public_drain.clone();
    let bounded_drain = public_drain.clone();
    let public_server = axum::serve(
        public_listener,
        public.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(async move { wait_for_drain(&mut public_drain).await });
    let operations_server = axum::serve(operations_listener, operations)
        .with_graceful_shutdown(async move { wait_for_drain(&mut operations_drain).await });
    complete_with_drain_timeout(
        async { tokio::try_join!(public_server, operations_server).map(|_| ()) },
        bounded_drain,
    )
    .await
}

async fn complete_with_drain_timeout(
    servers: impl Future<Output = std::io::Result<()>>,
    mut bounded_drain: tokio::sync::watch::Receiver<bool>,
) -> std::io::Result<()> {
    tokio::select! {
        result = servers => result,
        _ = async move {
            wait_for_drain(&mut bounded_drain).await;
            tokio::time::sleep(DRAIN_TIMEOUT).await;
        } => Ok(()),
    }
}

async fn wait_for_drain(receiver: &mut tokio::sync::watch::Receiver<bool>) {
    if *receiver.borrow() {
        return;
    }
    let _ = receiver.changed().await;
}

#[cfg(test)]
mod certification_tests {
    use std::time::Duration;

    use tokio::io::AsyncWriteExt;

    use super::*;

    #[tokio::test(start_paused = true)]
    async fn production_drain_releases_both_listeners_at_five_seconds() {
        let public_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let public_address = public_listener.local_addr().unwrap();
        let operations_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let operations_address = operations_listener.local_addr().unwrap();
        let state = AppState::disabled(crate::ServerConfig::loopback_default());
        let (shutdown, shutdown_receive) = tokio::sync::oneshot::channel();
        let server = tokio::spawn(serve_production(
            public_listener,
            operations_listener,
            state,
            async move {
                let _ = shutdown_receive.await;
            },
        ));
        tokio::task::yield_now().await;

        let mut public_connection = tokio::net::TcpStream::connect(public_address)
            .await
            .unwrap();
        let mut operations_connection = tokio::net::TcpStream::connect(operations_address)
            .await
            .unwrap();
        public_connection.write_all(b"GET /").await.unwrap();
        operations_connection.write_all(b"GET /").await.unwrap();
        shutdown.send(()).unwrap();
        tokio::task::yield_now().await;

        tokio::time::advance(DRAIN_TIMEOUT).await;
        tokio::task::yield_now().await;
        server.await.unwrap().unwrap();
        drop(public_connection);
        drop(operations_connection);

        let rebound_public = TcpListener::bind(public_address).await.unwrap();
        let rebound_operations = TcpListener::bind(operations_address).await.unwrap();
        drop((rebound_public, rebound_operations));
    }

    #[tokio::test(start_paused = true)]
    async fn noncooperative_servers_are_cut_off_at_the_exact_drain_boundary() {
        let (draining, receiver) = tokio::sync::watch::channel(false);
        let completion = tokio::spawn(complete_with_drain_timeout(
            std::future::pending::<std::io::Result<()>>(),
            receiver,
        ));
        draining.send(true).unwrap();
        tokio::task::yield_now().await;
        tokio::time::advance(Duration::from_secs(4)).await;
        tokio::task::yield_now().await;
        assert!(!completion.is_finished());
        tokio::time::advance(Duration::from_secs(1)).await;
        tokio::task::yield_now().await;
        completion.await.unwrap().unwrap();
    }
}
