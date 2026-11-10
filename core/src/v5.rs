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
/// bytes before the payload: start, length, control, seq + counter, serial
const HEADER_LEN: usize = 11;
/// bytes after the payload: checksum, end
const TRAILER_LEN: usize = 2;
/// longest payload we accept; the biggest real reply has 271 bytes
const MAX_PAYLOAD: usize = 512;

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

/// one frame from the logger whose envelope checked out; what's inside isn't
/// judged yet
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Frame {
    /// what kind of frame: reply, heartbeat, ...
    control: u16,
    /// the request's sequence byte, echoed back in replies
    seq: u8,
    /// serial of the logger that sent it, as received
    logger: u32,
    /// everything between the header and the checksum
    payload: Vec<u8>,
}

/// what [`split`] found at the front of the buffer
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Split {
    /// not a whole frame yet, wait for more bytes
    NeedMore,
    /// a whole frame; drop `used` bytes from the buffer
    Frame { frame: Frame, used: usize },
    /// bytes that aren't a frame; drop `used` bytes from the buffer
    Junk { used: usize, why: Skip },
}

/// why bytes or a frame were dropped
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Skip {
    /// bytes before a start byte
    NoStart,
    /// length longer than any real frame
    TooLong,
    /// no end byte where the length says
    BadEnd,
    /// checksum doesn't match
    BadChecksum,
}

/// cuts the first frame off the front of the bytes read so far
///
/// tcp has no messages, so one read can hold half a frame, or a reply with a
/// heartbeat stuck to it. Call this until it says `NeedMore`, dropping `used`
/// bytes after each answer.
pub(crate) fn split(buf: &[u8]) -> Split {
    if buf.is_empty() {
        return Split::NeedMore;
    }
    // nothing before the first start byte can be a frame
    let start = buf
        .iter()
        .position(|&byte| byte == START)
        .unwrap_or(buf.len());
    if start > 0 {
        return Split::Junk {
            used: start,
            why: Skip::NoStart,
        };
    }
    if buf.len() < 3 {
        return Split::NeedMore;
    }

    // from here on, a broken frame drops only its start byte: that a5 may have
    // been a data byte, and a real frame may start a few bytes later
    let payload_len = usize::from(u16::from_le_bytes([buf[1], buf[2]]));
    if payload_len > MAX_PAYLOAD {
        return Split::Junk {
            used: 1,
            why: Skip::TooLong,
        };
    }
    let len = HEADER_LEN + payload_len + TRAILER_LEN;
    if buf.len() < len {
        return Split::NeedMore;
    }
    if buf[len - 1] != END {
        return Split::Junk {
            used: 1,
            why: Skip::BadEnd,
        };
    }
    if checksum(&buf[1..len - 2]) != buf[len - 2] {
        return Split::Junk {
            used: 1,
            why: Skip::BadChecksum,
        };
    }

    Split::Frame {
        frame: Frame {
            control: u16::from_le_bytes([buf[3], buf[4]]),
            seq: buf[5],
            logger: u32::from_le_bytes([buf[7], buf[8], buf[9], buf[10]]),
            payload: buf[HEADER_LEN..len - TRAILER_LEN].to_vec(),
        },
        used: len,
    }
}

/// v5 checksum
fn checksum(bytes: &[u8]) -> u8 {
    bytes.iter().fold(0, |sum, &byte| sum.wrapping_add(byte))
}

#[cfg(test)]
mod tests;
