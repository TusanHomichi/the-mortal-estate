use serde::{Deserialize, Serialize};

/// Deterministic rules time measured in logical rounds.
///
/// This is deliberately independent of wall-clock seconds. The first authored
/// actor opportunity is [`LogicalTime::FIRST`]; zero is retained for initial
/// lifecycle state that must tick after the first completed logical round.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct LogicalTime(u64);

impl std::fmt::Display for LogicalTime {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

impl LogicalTime {
    pub const ZERO: Self = Self(0);
    pub const FIRST: Self = Self(1);

    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn value(self) -> u64 {
        self.0
    }

    pub fn saturating_add_rounds(self, rounds: u32) -> Self {
        Self(self.0.saturating_add(u64::from(rounds)))
    }

    pub fn elapsed_rounds_since(self, earlier: Self) -> u64 {
        self.0.saturating_sub(earlier.0)
    }

    pub fn is_multiple_of(self, rounds: u32) -> bool {
        rounds != 0 && self.0.is_multiple_of(u64::from(rounds))
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
    fn logical_time_multiple_checks_reject_zero_cadence() {
        assert!(LogicalTime::new(6).is_multiple_of(3));
        assert!(!LogicalTime::new(7).is_multiple_of(3));
        assert!(!LogicalTime::new(6).is_multiple_of(0));
    }
}
