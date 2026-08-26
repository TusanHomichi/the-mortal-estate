use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Default)]
struct Metrics {
    command_commits: AtomicU64,
    command_commit_failures: AtomicU64,
    system_commits: AtomicU64,
    system_commit_failures: AtomicU64,
    database_latency_micros: AtomicU64,
    database_observations: AtomicU64,
    database_failures: AtomicU64,
    facet_task_panics: AtomicU64,
    scheduler_skips: AtomicU64,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct MetricsSnapshot {
    pub command_commits: u64,
    pub command_commit_failures: u64,
    pub system_commits: u64,
    pub system_commit_failures: u64,
    pub database_latency_micros: u64,
    pub database_observations: u64,
    pub database_failures: u64,
    pub facet_task_panics: u64,
    pub scheduler_skips: u64,
}

fn metrics() -> &'static Metrics {
    static METRICS: OnceLock<Metrics> = OnceLock::new();
    METRICS.get_or_init(Metrics::default)
}

pub(crate) fn record_command_commit(success: bool, elapsed: std::time::Duration) {
    let metrics = metrics();
    metrics.command_commits.fetch_add(1, Ordering::Relaxed);
    if !success {
        metrics
            .command_commit_failures
            .fetch_add(1, Ordering::Relaxed);
        metrics.database_failures.fetch_add(1, Ordering::Relaxed);
    }
    record_database_elapsed(metrics, elapsed);
}

pub(crate) fn record_system_commit(success: bool, elapsed: std::time::Duration) {
    let metrics = metrics();
    metrics.system_commits.fetch_add(1, Ordering::Relaxed);
    if !success {
        metrics
            .system_commit_failures
            .fetch_add(1, Ordering::Relaxed);
        metrics.database_failures.fetch_add(1, Ordering::Relaxed);
    }
    record_database_elapsed(metrics, elapsed);
}

fn record_database_elapsed(metrics: &Metrics, elapsed: std::time::Duration) {
    let micros = elapsed.as_micros().min(u128::from(u64::MAX)) as u64;
    metrics
        .database_latency_micros
        .fetch_add(micros, Ordering::Relaxed);
    metrics
        .database_observations
        .fetch_add(1, Ordering::Relaxed);
}

pub(crate) fn record_database_failure() {
    metrics().database_failures.fetch_add(1, Ordering::Relaxed);
}

pub(crate) fn record_facet_task_panic() {
    metrics().facet_task_panics.fetch_add(1, Ordering::Relaxed);
}

pub(crate) fn record_scheduler_skip() {
    metrics().scheduler_skips.fetch_add(1, Ordering::Relaxed);
}

pub(crate) fn snapshot() -> MetricsSnapshot {
    let metrics = metrics();
    MetricsSnapshot {
        command_commits: metrics.command_commits.load(Ordering::Relaxed),
        command_commit_failures: metrics.command_commit_failures.load(Ordering::Relaxed),
        system_commits: metrics.system_commits.load(Ordering::Relaxed),
        system_commit_failures: metrics.system_commit_failures.load(Ordering::Relaxed),
        database_latency_micros: metrics.database_latency_micros.load(Ordering::Relaxed),
        database_observations: metrics.database_observations.load(Ordering::Relaxed),
        database_failures: metrics.database_failures.load(Ordering::Relaxed),
        facet_task_panics: metrics.facet_task_panics.load(Ordering::Relaxed),
        scheduler_skips: metrics.scheduler_skips.load(Ordering::Relaxed),
    }
}

pub fn init() {
    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .json()
        .finish();
    let _ = tracing::subscriber::set_global_default(subscriber);
}
