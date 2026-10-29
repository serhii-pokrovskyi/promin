//! models

use crate::Error;
use std::fmt;

/// deviceId - inverter serial
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DeviceId(String);

impl DeviceId {
    /// check text and validates
    pub fn new(id: &str) -> Result<Self, Error> {
        let allowed = |b: u8| b.is_ascii_alphanumeric() || b == b'-' || b == b'_';
        if (1..=32).contains(&id.len()) && id.bytes().all(allowed) {
            Ok(Self(id.to_owned()))
        } else {
            Err(Error::InvalidDeviceId(id.to_owned()))
        }
    }

    /// id as text
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for DeviceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests;
