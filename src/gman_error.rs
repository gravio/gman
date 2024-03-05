use core::fmt;
use std::error::Error;

#[derive(Debug)]
pub struct GManError {
    pub details: String,
}

impl GManError {
    pub fn new(msg: &str) -> GManError {
        GManError {
            details: msg.to_string(),
        }
    }
}

impl fmt::Display for GManError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.details)
    }
}

impl Error for GManError {}

unsafe impl Send for GManError {}

unsafe impl Sync for GManError {}
