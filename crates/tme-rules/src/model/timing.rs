use serde::{Deserialize, Serialize};

/// One authored action-cost unit currently lasts three seconds. This converts
/// durations, not a shared phase or a global gameplay pulse.
pub const ACTION_TIME_UNIT_MILLIS: u64 = 3_000;

/// Deterministic authoritative elapsed time with millisecond precision.
/// Whole-unit constructors remain useful for authored durations and simulation
/// examples; live deadlines preserve the offset of the action that starts them.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(from = "TimeMilliseconds", into = "TimeMilliseconds")]
pub struct LogicalTime(u64);

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TimeMilliseconds {
    milliseconds: u64,
}

impl From<TimeMilliseconds> for LogicalTime {
    fn from(value: TimeMilliseconds) -> Self {
        Self(value.milliseconds)
    }
}
impl From<LogicalTime> for TimeMilliseconds {
    fn from(value: LogicalTime) -> Self {
        Self {
            milliseconds: value.0,
        }
    }
}
impl std::fmt::Display for LogicalTime {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}ms", self.0)
    }
}
impl LogicalTime {
    pub const ZERO: Self = Self(0);
    pub const FIRST: Self = Self(ACTION_TIME_UNIT_MILLIS);

    /// A whole authored duration-unit position, used by deterministic callers.
    pub const fn new(units: u64) -> Self {
        Self(units.saturating_mul(ACTION_TIME_UNIT_MILLIS))
    }
    pub const fn from_millis(milliseconds: u64) -> Self {
        Self(milliseconds)
    }
    pub const fn as_millis(self) -> u64 {
        self.0
    }
    /// Completed whole units; never use this truncated view for live deadlines.
    pub const fn value(self) -> u64 {
        self.0 / ACTION_TIME_UNIT_MILLIS
    }
    pub fn saturating_add_millis(self, milliseconds: u64) -> Self {
        Self(self.0.saturating_add(milliseconds))
    }
    pub fn saturating_add_rounds(self, units: u32) -> Self {
        self.saturating_add_millis(u64::from(units).saturating_mul(ACTION_TIME_UNIT_MILLIS))
    }
    pub fn elapsed_rounds_since(self, earlier: Self) -> u64 {
        self.0.saturating_sub(earlier.0) / ACTION_TIME_UNIT_MILLIS
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActionCost(u32);

impl ActionCost {
    pub const FREE: Self = Self(0);
    pub const STANDARD: Self = Self(1);

    pub const fn from_positive_units(units: u32) -> Option<Self> {
        if units == 0 { None } else { Some(Self(units)) }
    }

    pub const fn units(self) -> u32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActorTimingState {
    pub ready_at: LogicalTime,
    pub tie_break_order: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorldTimingState {
    pub now: LogicalTime,
    pub next_tie_break_order: u64,
}

#[cfg(test)]
mod tests {
    use super::{ActionCost, LogicalTime};

    #[test]
    fn logical_time_addition_and_elapsed_rounds_are_saturating() {
        assert_eq!(LogicalTime::ZERO.value(), 0);
        assert!(LogicalTime::ZERO < LogicalTime::FIRST);
        assert_eq!(
            LogicalTime::new(5).saturating_add_rounds(2),
            LogicalTime::new(7)
        );
        assert_eq!(
            LogicalTime::new(u64::MAX).saturating_add_rounds(1),
            LogicalTime::new(u64::MAX)
        );
        assert_eq!(
            LogicalTime::new(7).elapsed_rounds_since(LogicalTime::new(5)),
            2
        );
        assert_eq!(
            LogicalTime::new(5).elapsed_rounds_since(LogicalTime::new(7)),
            0
        );
    }

    #[test]
    fn action_costs_are_finite_logical_round_counts() {
        assert_eq!(ActionCost::FREE.units(), 0);
        assert_eq!(ActionCost::STANDARD.units(), 1);
        assert_eq!(
            ActionCost::from_positive_units(3).map(ActionCost::units),
            Some(3)
        );
        assert_eq!(ActionCost::from_positive_units(0), None);
    }

    #[test]
    fn action_deadlines_preserve_sub_unit_offsets_and_refuse_old_scalar_time() {
        let start = LogicalTime::from_millis(4_127);
        assert_eq!(start.saturating_add_rounds(1).as_millis(), 7_127);
        assert_eq!(
            LogicalTime::from_millis(7_126).elapsed_rounds_since(start),
            0
        );
        assert_eq!(
            LogicalTime::from_millis(7_127).elapsed_rounds_since(start),
            1
        );
        assert_eq!(
            serde_json::to_string(&start).unwrap(),
            r#"{"milliseconds":4127}"#
        );
        assert!(serde_json::from_str::<LogicalTime>("1").is_err());
        assert!(serde_json::from_str::<LogicalTime>(r#"{"milliseconds":4127,"round":1}"#).is_err());
    }
}
