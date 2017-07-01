use std::error::Error;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;

#[derive(Clone, Debug)]
pub struct RejectError;

impl Display for RejectError {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "Rejected command")
    }
}
impl Error for RejectError {
    fn description(&self) -> &str {
        "Command run got reject, because the circuit is open"
    }
}
unsafe impl Send for RejectError {}
unsafe impl Sync for RejectError {}