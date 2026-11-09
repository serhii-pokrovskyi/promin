//! solarman v5, the envelope the logger stick puts around modbus rtu frames
//!
//! frame layout as documented by pysolarmanv5:
//! <https://pysolarmanv5.readthedocs.io/en/latest/solarmanv5_protocol.html>

#![cfg_attr(not(test), expect(dead_code, reason = "library"))]

use std::num::NonZeroU32;

use crate::Error;
use crate::modbus::ReadRequest;

const START: u8 = 0xA5;
const END: u8 = 0x15;
/// code of client's request
const REQUEST: u16 = 0x4510;
/// payload frame
const INVERTER: u8 = 0x02;

/// logger's serial
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct LoggerSerial(NonZeroU32);

impl LoggerSerial {
    /// rejects 0
    pub(crate) fn new(serial: u32) -> Result<Self, Error> {
        NonZeroU32::new(serial)
            .map(Self)
            .ok_or(Error::InvalidRequest("logger serial must not be null"))
    }

    /// serial as number
    pub(crate) fn get(self) -> u32 {
        self.0.get()
    }
}

/// 36 byte v5 frame
pub(crate) fn request_frame(req: &ReadRequest, logger: LoggerSerial, seq: u8) -> [u8; 36] {
    const PAYLOAD_LEN: u16 = 15 + 8;
    let mut frame = [0; 36];

    frame[0] = START;
    frame[1..3].copy_from_slice(&PAYLOAD_LEN.to_le_bytes());
    frame[3..5].copy_from_slice(&REQUEST.to_le_bytes());
    frame[5] = seq;
    // frame[6] is logger counter
    frame[7..11].copy_from_slice(&logger.get().to_le_bytes());
    frame[11] = INVERTER;
    // 12..26: sensor type and the three time fields, all 0
    frame[26..34].copy_from_slice(&req.rtu_frame());
    frame[34] = checksum(&frame[1..34]);
    frame[35] = END;
    frame
}

/// v5 checksum
fn checksum(bytes: &[u8]) -> u8 {
    bytes.iter().fold(0, |sum, &byte| sum.wrapping_add(byte))
}

#[cfg(test)]
mod tests;
