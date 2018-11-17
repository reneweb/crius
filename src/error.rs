use std::fmt;
use std::error::Error;

/// This error type describes the possible failures that can occur
/// while attempting to run a circuit breaker command.
pub enum CriusError {
    /// Error variant returned in case of an open breaker.
    ExecutionRejected,

    /// Error variant returned in case of invalid configuration (e.g.
    /// parameters that cause duration calculations to overflow).
    InvalidConfig,
}

const REJECTED: &str = "Rejected command execution due to open breaker";
const INVALID: &str = "Provided circuit breaker configuration was invalid";

impl fmt::Display for CriusError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            CriusError::ExecutionRejected => write!(f, "{}", REJECTED),
            CriusError::InvalidConfig => write!(f, "{}", INVALID),
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
            CriusError::ExecutionRejected => REJECTED,
            CriusError::InvalidConfig => INVALID,
        }
    }
}
