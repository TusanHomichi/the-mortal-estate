use std::sync::Arc;
use std::time::Duration;

use tokio::sync::mpsc;
use tokio::time::{MissedTickBehavior, interval};

use crate::facet::FacetRequest;

/// Wake the world owner to check deadlines. This interval never defines game time.
const DEADLINE_CHECK_INTERVAL: Duration = Duration::from_millis(25);

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
        let mut clock = interval(DEADLINE_CHECK_INTERVAL);
        clock.set_missed_tick_behavior(MissedTickBehavior::Skip);
        clock.tick().await;
        loop {
            clock.tick().await;
            match sender.try_send(FacetRequest::CheckDeadlines) {
                Ok(()) => {}
                Err(mpsc::error::TrySendError::Full(_)) => {
                    crate::telemetry::record_scheduler_skip();
                }
                Err(mpsc::error::TrySendError::Closed(_)) => break,
            }
        }
    })
}

/// Recovered simulation time is rebased on one monotonic server clock. Downtime
/// pauses the world; it never clears a character's remaining cooldown.
pub(super) struct FacetClock {
    origin: tokio::time::Instant,
    logical_origin: tme_rules::LogicalTime,
}
impl FacetClock {
    pub(super) fn new(logical_origin: tme_rules::LogicalTime) -> Self {
        Self {
            origin: tokio::time::Instant::now(),
            logical_origin,
        }
    }
    pub(super) fn now(&self) -> tme_rules::LogicalTime {
        self.logical_origin.saturating_add_millis(
            u64::try_from(self.origin.elapsed().as_millis()).unwrap_or(u64::MAX),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test(start_paused = true)]
    async fn clock_preserves_offsets_and_rebases_recovery_without_clearing_cooldowns() {
        let clock = FacetClock::new(tme_rules::LogicalTime::from_millis(4_127));
        tokio::time::advance(Duration::from_millis(1_173)).await;
        assert_eq!(clock.now().as_millis(), 5_300);
        let checkpoint_time = clock.now();
        let ready_at = tme_rules::LogicalTime::from_millis(7_127);
        tokio::time::advance(Duration::from_secs(60)).await;
        let recovered = FacetClock::new(checkpoint_time);
        assert_eq!(ready_at.as_millis() - recovered.now().as_millis(), 1_827);
        tokio::time::advance(Duration::from_millis(1_826)).await;
        assert!(recovered.now() < ready_at);
        tokio::time::advance(Duration::from_millis(1)).await;
        assert_eq!(recovered.now(), ready_at);
    }
    #[tokio::test(start_paused = true)]
    async fn housekeeping_only_requests_a_deadline_check() {
        let (sender, mut receiver) = mpsc::channel(4);
        let task = spawn(sender, None);
        tokio::task::yield_now().await;
        assert!(receiver.try_recv().is_err());
        tokio::time::advance(DEADLINE_CHECK_INTERVAL).await;
        tokio::task::yield_now().await;
        assert!(matches!(
            receiver.try_recv(),
            Ok(FacetRequest::CheckDeadlines)
        ));
        task.abort();
    }
}
