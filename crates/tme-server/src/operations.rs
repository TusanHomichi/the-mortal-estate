use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router, extract::State};
use serde::Serialize;
use tme_protocol as wire;

use crate::AppState;

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct OperationsStatus {
    status: &'static str,
    gameplay_ready: bool,
    draining: bool,
    open_connections: usize,
    preauth_connections: usize,
    maximum_mailbox_depth: usize,
    mailbox_capacity: usize,
    restore_fence_epoch: u64,
    protocol_major: u16,
    protocol_minor: u16,
    control_api_version: u16,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/internal/status", get(status))
        .route("/internal/metrics", get(metrics))
        .with_state(state)
}

async fn status(State(state): State<AppState>) -> Response {
    let value = state.operations_status().await;
    (
        if value.gameplay_ready {
            StatusCode::OK
        } else {
            StatusCode::SERVICE_UNAVAILABLE
        },
        Json(value),
    )
        .into_response()
}

async fn metrics(State(state): State<AppState>) -> Response {
    let value = state.operations_status().await;
    let counters = crate::telemetry::snapshot();
    let body = format!(
        concat!(
            "# TYPE tme_gameplay_ready gauge\n",
            "tme_gameplay_ready {}\n",
            "# TYPE tme_draining gauge\n",
            "tme_draining {}\n",
            "# TYPE tme_connections gauge\n",
            "tme_connections {}\n",
            "# TYPE tme_preauth_connections gauge\n",
            "tme_preauth_connections {}\n",
            "# TYPE tme_mailbox_maximum_depth gauge\n",
            "tme_mailbox_maximum_depth {}\n",
            "# TYPE tme_mailbox_capacity gauge\n",
            "tme_mailbox_capacity {}\n",
            "# TYPE tme_command_commits_total counter\n",
            "tme_command_commits_total {}\n",
            "# TYPE tme_command_commit_failures_total counter\n",
            "tme_command_commit_failures_total {}\n",
            "# TYPE tme_system_commits_total counter\n",
            "tme_system_commits_total {}\n",
            "# TYPE tme_system_commit_failures_total counter\n",
            "tme_system_commit_failures_total {}\n",
            "# TYPE tme_database_latency_microseconds_total counter\n",
            "tme_database_latency_microseconds_total {}\n",
            "# TYPE tme_database_observations_total counter\n",
            "tme_database_observations_total {}\n",
            "# TYPE tme_database_failures_total counter\n",
            "tme_database_failures_total {}\n",
            "# TYPE tme_facet_task_panics_total counter\n",
            "tme_facet_task_panics_total {}\n",
            "# TYPE tme_scheduler_skips_total counter\n",
            "tme_scheduler_skips_total {}\n",
            "# TYPE tme_restore_fence_epoch gauge\n",
            "tme_restore_fence_epoch {}\n"
        ),
        usize::from(value.gameplay_ready),
        usize::from(value.draining),
        value.open_connections,
        value.preauth_connections,
        value.maximum_mailbox_depth,
        value.mailbox_capacity,
        counters.command_commits,
        counters.command_commit_failures,
        counters.system_commits,
        counters.system_commit_failures,
        counters.database_latency_micros,
        counters.database_observations,
        counters.database_failures,
        counters.facet_task_panics,
        counters.scheduler_skips,
        value.restore_fence_epoch,
    );
    ([(header::CONTENT_TYPE, "text/plain; version=0.0.4")], body).into_response()
}

impl AppState {
    async fn operations_status(&self) -> OperationsStatus {
        let backend_ready = self
            .inner
            .backend
            .as_ref()
            .is_some_and(|backend| backend.gameplay_ready())
            && !self.is_draining();
        let (restore_fence_epoch, database_ready) = match self.inner.backend.as_ref() {
            Some(backend) => match backend.restore_fence_epoch().await {
                Ok(value) => (value, true),
                Err(_) => {
                    crate::telemetry::record_database_failure();
                    (0, false)
                }
            },
            None => (0, false),
        };
        let gameplay_ready = backend_ready && database_ready;
        OperationsStatus {
            status: if gameplay_ready {
                "ready"
            } else {
                "unavailable"
            },
            gameplay_ready,
            draining: self.is_draining(),
            open_connections: self.inner.open.load(std::sync::atomic::Ordering::Acquire),
            preauth_connections: self
                .inner
                .preauth
                .load(std::sync::atomic::Ordering::Acquire),
            maximum_mailbox_depth: self
                .inner
                .backend
                .as_ref()
                .map_or(0, |backend| backend.maximum_mailbox_depth()),
            mailbox_capacity: crate::config::FACET_MAILBOX_CAPACITY,
            restore_fence_epoch,
            protocol_major: wire::PROTOCOL_MAJOR,
            protocol_minor: wire::PROTOCOL_MINOR,
            control_api_version: wire::CONTROL_API_VERSION,
        }
    }
}
