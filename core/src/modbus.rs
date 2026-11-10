//! modbus rtu, read only

#![cfg_attr(not(test), expect(dead_code, reason = "forst used later"))]

use crate::Error;
use crate::error::{ExceptionCode, RtuError, RtuFault};

/// witch register table to read
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReadKind {
    /// holding registers, 0x03
    Holding,
    ///input registers, 0x04
    Input,
}

impl ReadKind {
    /// modbus function code for this read
    pub fn function_code(self) -> u8 {
        match self {
            Self::Holding => 0x03,
            Self::Input => 0x04,
        }
    }
}

/// one modbus read: `count` registers from `start`, on device `slave`.
///
/// the fields are private, so [`ReadRequest::new`] and its shortcuts are the
/// only way to get one. That's what keeps promin read-only.
///
/// ```
/// use promin_core::modbus::ReadRequest;
///
/// // Deye keeps the inverter serial in holding registers 0x0003..=0x0007.
/// let req = ReadRequest::holding(1, 0x0003, 5)?;
/// assert_eq!(req.count(), 5);
/// # Ok::<(), promin_core::Error>(())
/// ```
///
/// ```compile_fail
/// use promin_core::modbus::{ReadKind, ReadRequest};
///
/// let req = ReadRequest { slave: 1, kind: ReadKind::Holding, start: 0, count: 1 };
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ReadRequest {
    slave: u8,
    kind: ReadKind,
    start: u16,
    count: u16,
}

impl ReadRequest {
    // frame max count
    const MAX_COUNT: u16 = 125;

    /// checks limits and builds read
    pub fn new(slave: u8, kind: ReadKind, start: u16, count: u16) -> Result<Self, Error> {
        if !(1..=247).contains(&slave) {
            return Err(Error::InvalidRequest("slave id must be 1..=247"));
        }
        if !(1..=Self::MAX_COUNT).contains(&count) {
            return Err(Error::InvalidRequest("count must be 1..=125"));
        }

        if start.checked_add(count - 1).is_none() {
            return Err(Error::InvalidRequest("block runs past register 0xffff"));
        }
        Ok(Self {
            slave,
            kind,
            start,
            count,
        })
    }

    /// [`ReadRequest::new`] for holding registers
    ///
    /// # Errors
    ///
    /// Same as [`ReadRequest::new`]
    pub fn holding(slave: u8, start: u16, count: u16) -> Result<Self, Error> {
        Self::new(slave, ReadKind::Holding, start, count)
    }

    /// [`ReadRequest::new`] for input registers
    ///
    /// # Errors
    ///
    /// same as [`ReadRequest::new`]
    pub fn input(slave: u8, start: u16, count: u16) -> Result<Self, Error> {
        Self::new(slave, ReadKind::Input, start, count)
    }

    /// modbus device address
    pub fn slave(&self) -> u8 {
        self.slave
    }

    /// which register table this reads
    pub fn kind(&self) -> ReadKind {
        self.kind
    }

    /// first register
    pub fn start(&self) -> u16 {
        self.start
    }

    /// number of registers
    pub fn count(&self) -> u16 {
        self.count
    }

    /// 8-byte rtu frame
    pub(crate) fn rtu_frame(&self) -> [u8; 8] {
        let [start_hi, start_lo] = self.start.to_be_bytes();
        let [count_hi, count_lo] = self.count.to_be_bytes();

        let body = [
            self.slave,
            self.kind.function_code(),
            start_hi,
            start_lo,
            count_hi,
            count_lo,
        ];
        let mut frame = [0; 8];
        frame[..6].copy_from_slice(&body);
        frame[6..].copy_from_slice(&crc16(&body).to_le_bytes());
        frame
    }

    /// checks an rtu reply against this request and returns the register values
    pub(crate) fn parse_reply(&self, frame: &[u8]) -> Result<Vec<u16>, Error> {
        let function = self.kind.function_code();
        // shortest reply is an exception: slave, function | 0x80, code, crc
        if frame.len() < 5 {
            return Err(rtu(RtuFault::TooShort { len: frame.len() }));
        }
        if frame[0] != self.slave {
            return Err(rtu(RtuFault::Slave {
                expected: self.slave,
                got: frame[0],
            }));
        }
        if frame[1] == (function | 0x80) {
            check_crc(frame, 3)?;
            return Err(Error::Exception(ExceptionCode(frame[2])));
        }
        if frame[1] != function {
            return Err(rtu(RtuFault::Function {
                expected: function,
                got: frame[1],
            }));
        }
        let data_len = usize::from(self.count) * 2;
        if usize::from(frame[2]) != data_len {
            return Err(rtu(RtuFault::ByteCount {
                expected: data_len,
                got: usize::from(frame[2]),
            }));
        }
        check_crc(frame, 3 + data_len)?;
        let (pairs, _) = frame[3..3 + data_len].as_chunks::<2>();
        Ok(pairs.iter().map(|&pair| u16::from_be_bytes(pair)).collect())
    }
}

/// checks the crc that follows the first `body` bytes
///
/// some deye units send `00 00` after the crc (the "double crc"). Exactly that
/// is allowed; any other extra bytes are an error
fn check_crc(frame: &[u8], body: usize) -> Result<(), Error> {
    let len = body + 2;
    let double_crc = frame.len() == len + 2 && frame[len..] == [0, 0];
    if frame.len() != len && !double_crc {
        return Err(rtu(RtuFault::Length {
            expected: len,
            got: frame.len(),
        }));
    }
    let expected = crc16(&frame[..body]);
    let got = u16::from_le_bytes([frame[body], frame[body + 1]]);
    if got != expected {
        return Err(rtu(RtuFault::Crc { expected, got }));
    }
    Ok(())
}

/// crc-16/modbus, start at 0xffff, polynormal 0xa001 (0x8005 bit reversed)
/// on write it goes low byte first
pub(crate) fn crc16(bytes: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for &byte in bytes {
        crc ^= u16::from(byte);
        for _ in 0..8 {
            let carry = crc & 1;
            crc >>= 1;
            if carry != 0 {
                crc ^= 0xA001;
            }
        }
    }
    crc
}

fn rtu(fault: RtuFault) -> Error {
    Error::Rtu(RtuError(fault))
}

#[cfg(test)]
mod tests;
