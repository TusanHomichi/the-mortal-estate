use std::sync::Arc;
use std::time::Duration;

use tokio::sync::mpsc;
use tokio::time::{MissedTickBehavior, interval};

use crate::facet::FacetRequest;

/// The wall-clock cadence at which authoritative gameplay boundaries are struck.
///
/// Owner ruling **D5** (2026-08-19) sets one authoritative gameplay pulse at
/// **3.0 seconds**, and states explicitly that the one-second statement is not
/// product authority. Player readiness, automatic actors, spell preparation,
/// recovery, and pulse-owned environmental processes all derive from this beat
/// by counting rounds; nothing else reads a clock of its own.
///
/// This is the whole cadence — one value, in the one place that owns *when* a
/// beat is struck. It may not be changed by editing this constant: a cadence
/// change requires an explicit owner ruling backed by a side-by-side play-feel
/// test, and `docs/boundary-map.md` §2.1 changes with it.
pub const GAMEPLAY_PULSE: Duration = Duration::from_millis(3_000);

struct SchedulerReadinessGuard(Option<Arc<crate::postgres::GameplayReadiness>>);

impl Drop for SchedulerReadinessGuard {
    fn drop(&mut self) {
        if let Some(readiness) = &self.0 {
            readiness.fail();
        }
    }
}

pub(super) fn spawn(
    sender: mpsc::Sender<FacetRequest>,
    readiness: Option<Arc<crate::postgres::GameplayReadiness>>,
) -> tokio::task::JoinHandle<()> {
    let readiness_guard = SchedulerReadinessGuard(readiness);
    tokio::spawn(async move {
        let _readiness_guard = readiness_guard;
        let mut clock = interval(GAMEPLAY_PULSE);
        clock.set_missed_tick_behavior(MissedTickBehavior::Skip);
        clock.tick().await;
        loop {
            clock.tick().await;
            match sender.try_send(FacetRequest::Tick) {
                Ok(()) => {}
                Err(mpsc::error::TrySendError::Full(_)) => {
                    crate::telemetry::record_scheduler_skip();
                }
                Err(mpsc::error::TrySendError::Closed(_)) => break,
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// D5's ruled value, restated so a silent edit to `GAMEPLAY_PULSE` fails
    /// here rather than reaching players as a changed game feel.
    #[test]
    fn the_gameplay_pulse_is_the_ruled_three_seconds() {
        assert_eq!(Duration::from_millis(3_000), GAMEPLAY_PULSE);
    }

    #[tokio::test(start_paused = true)]
    async fn scheduler_strikes_one_boundary_per_gameplay_pulse() {
        let (sender, mut receiver) = mpsc::channel(4);
        let scheduler = spawn(sender, None);
        tokio::task::yield_now().await;
        assert!(
            receiver.try_recv().is_err(),
            "a boundary was struck before one pulse elapsed"
        );

        // The cadence is the pulse, not a second: a full second of wall time is
        // a third of a beat and must strike nothing.
        tokio::time::advance(Duration::from_secs(1)).await;
        tokio::task::yield_now().await;
        assert!(
            receiver.try_recv().is_err(),
            "one elapsed second struck a boundary; the cadence is not one second"
        );

        tokio::time::advance(GAMEPLAY_PULSE - Duration::from_secs(1)).await;
        tokio::task::yield_now().await;
        assert!(
            matches!(receiver.try_recv(), Ok(FacetRequest::Tick)),
            "one elapsed pulse struck no boundary"
        );
        assert!(
            receiver.try_recv().is_err(),
            "one elapsed pulse struck more than one boundary"
        );

        // A second pulse strikes exactly once more, and a partial pulse on top
        // of it strikes nothing further.
        tokio::time::advance(GAMEPLAY_PULSE).await;
        tokio::task::yield_now().await;
        assert!(
            matches!(receiver.try_recv(), Ok(FacetRequest::Tick)),
            "the second pulse struck no boundary"
        );
        assert!(receiver.try_recv().is_err());

        tokio::time::advance(GAMEPLAY_PULSE / 2).await;
        tokio::task::yield_now().await;
        assert!(
            receiver.try_recv().is_err(),
            "half a pulse struck a boundary"
        );

        scheduler.abort();
    }
}
