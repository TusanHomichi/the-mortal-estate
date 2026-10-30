//! The harness against a real server: every bot gets in and plays, and
//! everything it measures is there. The world steps every five
//! milliseconds here, so a couple of seconds is a good stretch of play.

use std::path::Path;
use std::time::Duration;
use tme_bots::{Plan, measure};

#[tokio::test(flavor = "multi_thread")]
async fn every_bot_plays_and_every_intent_is_answered() {
    let content = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
    let plan = Plan {
        bots: 8,
        length: Duration::from_secs(2),
        step: Duration::from_millis(5),
    };
    let report = measure(&content, plan).await.expect("the run");
    assert!(report.failed.is_empty(), "{report}");
    assert_eq!(report.played, 8, "{report}");
    assert!(!report.steps.is_empty(), "{report}");
    assert!(report.latency.len() >= 8, "every bot acts: {report}");
    assert_eq!(report.unanswered, 0, "{report}");
}
