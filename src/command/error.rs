use std::fmt;
use std::error::Error;

/// This error type describes the possible failures that can occur
/// while attempting to run a circuit breaker command.
pub enum CriusError {
    /// Error type returned in case of an open breaker.
    ExecutionRejected,
}

impl fmt::Display for CriusError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            CriusError::ExecutionRejected =>
                write!(f, "Rejected command execution due to open breaker")
        }

    }
}

impl fmt::Debug for CriusError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        <Self as fmt::Display>::fmt(self, f)
    }
}

impl Error for CriusError {
    fn description(&self) -> &str {
        match *self {
            CriusError::ExecutionRejected =>
                "Rejected command execution due to open breaker"
        }
    }
}
