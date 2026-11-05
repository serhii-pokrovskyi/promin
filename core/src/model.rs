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
    /// empty snapshot
    pub fn new() -> Self {
        Self::default()
    }

    /// adds words read from consecutive registers, first one at `start`
    pub fn push_block(&mut self, start: u16, words: Vec<u16>) {
        self.blocks.push(RegisterBlock { start, words });
    }

    /// value of one register, None if no block has it; first block wins on overlap
    pub fn get(&self, addr: u16) -> Option<u16> {
        self.blocks.iter().find_map(|block| {
            let offset = addr.checked_sub(block.start)?;
            block.words.get(usize::from(offset)).copied()
        })
    }

    /// blocks in the order they were added
    pub fn blocks(&self) -> &[RegisterBlock] {
        &self.blocks
    }
}

/// words from one read, starting at `start`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisterBlock {
    start: u16,
    words: Vec<u16>,
}

impl RegisterBlock {
    /// address of the first register
    pub fn start(&self) -> u16 {
        self.start
    }

    /// register values, one per address
    pub fn words(&self) -> &[u16] {
        &self.words
    }
}

/// unix time millis
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UnixMillis(pub i64);

/// redings model
#[derive(Debug, Clone, PartialEq)]
pub struct Sample {
    /// device
    pub device: DeviceId,
    /// time
    pub at: UnixMillis,
    /// pv inputs
    pub pv: Vec<PvInput>,
    /// battery
    pub battery: Battery,
    /// grid
    pub grid: Grid,
    /// load
    pub load: Load,
    /// inverter
    pub inverter: Inverter,
    /// energy
    pub energy: Energy,
}

impl Sample {
    pub fn new(device: DeviceId, at: UnixMillis) -> Self {
        Self {
            device,
            at,
            pv: Vec::new(),
            battery: Battery::default(),
            grid: Grid::default(),
            load: Load::default(),
            inverter: Inverter::default(),
            energy: Energy::default(),
        }
    }
}

/// one pv input
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PvInput {
    /// voltage, v
    pub voltage_v: Option<f64>,
    /// current, a
    pub current_a: Option<f64>,
    /// power, w
    pub power_w: Option<f64>,
}

/// battery
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Battery {
    /// charge %
    pub soc_pct: Option<f64>,
    /// voltage, v
    pub voltage_v: Option<f64>,
    /// current, a
    pub current_a: Option<f64>,
    /// power, w
    pub power_w: Option<f64>,
    /// temperature, c
    pub temperature_c: Option<f64>,
}

/// grid
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Grid {
    /// power, w
    pub power_w: Option<f64>,
    /// voltage, v
    pub voltage_v: Option<f64>,
    /// frequency, hz
    pub frequency_hz: Option<f64>,
}

/// house load
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Load {
    /// power, w
    pub power_w: Option<f64>,
}

/// inverter
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Inverter {
    /// state
    pub state: Option<InverterState>,
    /// dc temp
    pub dc_temp_c: Option<f64>,
    /// ac temp
    pub ac_temp_c: Option<f64>,
}

///inverter global state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InverterState {
    /// no converting power
    Standby,
    /// running test
    SelfTest,
    /// normal work
    Normal,
    /// working with alarm
    Alarm,
    /// sopped with fault
    Fault,
    /// unknown state
    Unknown(u16),
}

/// energy counters, kwh
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Energy {
    /// pv total
    pub pv_total_kwh: Option<f64>,
    /// pv today
    pub pv_today_kwh: Option<f64>,
    /// grid impport total
    pub grid_import_total_kwh: Option<f64>,
    /// grid import today
    pub grid_import_today_kwh: Option<f64>,
    /// grid export total
    pub grid_export_total_kwh: Option<f64>,
    /// grid export today
    pub grid_export_today_kwh: Option<f64>,
    /// batery charge total
    pub battery_charge_total_kwh: Option<f64>,
    /// battery charge today
    pub battery_charge_today_kwh: Option<f64>,
    /// battery discharge total
    pub battery_discharge_total_kwh: Option<f64>,
    /// battery discharge today
    pub battery_discharge_today_kwh: Option<f64>,
    /// house load used total
    pub load_total_kwh: Option<f64>,
    /// house load used today
    pub load_today_kwh: Option<f64>,
}

#[cfg(test)]
mod tests;
