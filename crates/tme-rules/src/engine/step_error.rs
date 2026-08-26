//! The one error a step returns.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepError {
    message: String,
}

impl StepError {
    pub(in crate::engine) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
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
