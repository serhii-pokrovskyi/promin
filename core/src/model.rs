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

/// Raw register
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RegisterSnapshot {
    blocks: Vec<RegisterBlock>,
}

impl RegisterSnapshot {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push_block(&mut self, start: u16, words: Vec<u16>) {
        self.blocks.push(RegisterBlock { start, words });
    }

    pub fn get(&self, addr: u16) -> Option<u16> {
        self.blocks.iter().find_map(|block| {
            let offset = addr.checked_sub(block.start)?;
            block.words.get(usize::from(offset)).copied()
        })
    }

    pub fn blocks(&self) -> &[RegisterBlock] {
        &self.blocks
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisterBlock {
    start: u16,
    words: Vec<u16>,
}

impl RegisterBlock {
    pub fn start(&self) -> u16 {
        self.start
    }

    pub fn words(&self) -> &[u16] {
        &self.words
    }
}

#[cfg(test)]
mod tests;
