//! Library behind promin, a read-only local monitor for Deye solar inverters.
//!
//! Still in early development, nothing to use here yet.

mod error;
pub mod modbus;
pub mod model;
mod v5;

pub use error::{Error, ExceptionCode, RtuError};
