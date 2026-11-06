use std::fmt;

/// Represents lib error
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// Cant carry read request
    InvalidRequest(&'static str),
    /// text cant be a  DeviceId
    InvalidDeviceId(String),
    /// inverter refuesed to read
    Exception(ExceptionCode),
    /// something wrong with reply
    Rtu(RtuError),
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRequest(reason) => {
                write!(f, "invalid read request: {reason}")
            }
            Self::InvalidDeviceId(id) => {
                write!(
                    f,
                    "invalid device id {id:?}: use 1 to 32 ASCII letters, digits, '-' or '_'"
                )
            }
            Self::Exception(code) => {
                write!(f, "modbus exception: {code}")
            }
            Self::Rtu(err) => {
                write!(f, "malformed modbus reply: {err}")
            }
        }
    }
}

/// code in modbus exception reply
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExceptionCode(pub(crate) u8);

impl ExceptionCode {
    /// raw code
    pub fn code(self) -> u8 {
        self.0
    }
}

impl fmt::Display for ExceptionCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self.0 {
            0x01 => "illegal function",
            0x02 => "illegal data address",
            0x03 => "illegal data value",
            0x04 => "server device failure",
            0x05 => "acknowledge",
            0x06 => "server device busy",
            0x08 => "memory parity error",
            0x0A => "gateway path unavailable",
            0x0B => "gateway target device failed to respond",
            _ => "unknown",
        };
        write!(f, "{name} ({:#04x})", self.0)
    }
}

/// rtu error reply reason
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RtuError(pub(crate) RtuFault);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RtuFault {
    TooShort { len: usize },
    Slave { expected: u8, got: u8 },
    Function { expected: u8, got: u8 },
    ByteCount { expected: usize, got: usize },
    Length { expected: usize, got: usize },
    Crc { expected: u16, got: u16 },
}

impl fmt::Display for RtuError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            RtuFault::TooShort { len } => {
                write!(f, "only {len} bytes")
            }
            RtuFault::Slave { expected, got } => {
                write!(f, "from slave {got}, expected {expected}")
            }
            RtuFault::Function { expected, got } => {
                write!(f, "function {got:#04x}, expected {expected:#04x}")
            }
            RtuFault::ByteCount { expected, got } => {
                write!(f, "{got} data bytes, expected {expected}")
            }
            RtuFault::Length { expected, got } => {
                write!(
                    f,
                    "{got} bytes, expected {expected} (or {} ending in 00 00)",
                    expected + 2
                )
            }
            RtuFault::Crc { expected, got } => {
                write!(f, "crc {got:#06x}, expected {expected:#06x}")
            }
        }
    }
}

#[cfg(test)]
mod tests;
