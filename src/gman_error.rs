use core::fmt;
use std::error::Error;

#[derive(Debug)]
pub struct GravioError {
    pub details: String,
}

impl GravioError {
    pub fn new(msg: &str) -> GravioError {
        GravioError {
            details: msg.to_string(),
        }
    }
}

impl fmt::Display for GravioError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.details)
    }
}

impl Error for GravioError {
    fn description(&self) -> &str {
        &self.details
    }
}
