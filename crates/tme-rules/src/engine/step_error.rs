//! The one error a step returns.

use std::fmt;

use crate::view::ActionBlockedReasonV1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepError {
    message: String,
    action_blocked_reason: Option<ActionBlockedReasonV1>,
}

impl StepError {
    pub(in crate::engine) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            action_blocked_reason: None,
        }
    }

    pub(in crate::engine) fn blocked(
        reason: ActionBlockedReasonV1,
        message: impl Into<String>,
    ) -> Self {
        Self {
            message: message.into(),
            action_blocked_reason: Some(reason),
        }
    }

    pub(in crate::engine) fn action_blocked_reason(&self) -> Option<ActionBlockedReasonV1> {
        self.action_blocked_reason
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for StepError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.message)
    }
}

impl std::error::Error for StepError {}
