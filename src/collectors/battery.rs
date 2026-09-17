use crate::model::BatteryData;
use std::mem;
use windows_sys::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};

/// `BatteryFlag` bit meaning "no system battery" (desktop PCs).
const NO_SYSTEM_BATTERY: u8 = 0x80;
/// `BatteryFlag`/`BatteryLifePercent` sentinel meaning "unable to read".
const UNKNOWN: u8 = 0xFF;
/// `BatteryFlag` bit meaning the battery is currently charging.
const CHARGING: u8 = 0x08;
/// `BatteryLifeTime` sentinel meaning "unknown, or on AC power".
const UNKNOWN_TIME: u32 = u32::MAX;

pub struct BatteryCollector {
    history: Vec<f64>,
    max_history: usize,
}

impl BatteryCollector {
    pub fn new(max_history: usize) -> Self {
        Self {
            history: Vec::new(),
            max_history,
        }
    }

    pub fn collect(&mut self) -> BatteryData {
        let mut status: SYSTEM_POWER_STATUS = unsafe { mem::zeroed() };
        let ok = unsafe { GetSystemPowerStatus(&mut status) };
        if ok == 0 {
            return BatteryData::default();
        }

        let mut data = battery_data_from_status(&status);
        if data.is_present {
            self.history.push(data.percent);
            if self.history.len() > self.max_history {
                self.history.remove(0);
            }
            data.history = self.history.clone();
        }
        data
    }
}

/// Pure interpretation of a `SYSTEM_POWER_STATUS` reading, split out from
/// `collect()` so it's testable without calling into Win32 (the struct's
/// fields are all public, so tests can build one by hand).
fn battery_data_from_status(status: &SYSTEM_POWER_STATUS) -> BatteryData {
    // Also catches the fully-unknown sentinel (0xFF), which happens to carry
    // this bit too -- that's fine, it's safer to hide the panel than to
    // fabricate a reading for a battery we can't actually read.
    if status.BatteryFlag & NO_SYSTEM_BATTERY != 0 {
        return BatteryData::default();
    }

    BatteryData {
        is_present: true,
        percent: if status.BatteryLifePercent == UNKNOWN {
            0.0
        } else {
            status.BatteryLifePercent as f64
        },
        ac_online: status.ACLineStatus == 1,
        charging: status.BatteryFlag != UNKNOWN && status.BatteryFlag & CHARGING != 0,
        seconds_remaining: if status.BatteryLifeTime == UNKNOWN_TIME {
            None
        } else {
            Some(status.BatteryLifeTime)
        },
        history: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn status(
        ac_line: u8,
        battery_flag: u8,
        life_percent: u8,
        life_time: u32,
    ) -> SYSTEM_POWER_STATUS {
        SYSTEM_POWER_STATUS {
            ACLineStatus: ac_line,
            BatteryFlag: battery_flag,
            BatteryLifePercent: life_percent,
            SystemStatusFlag: 0,
            BatteryLifeTime: life_time,
            BatteryFullLifeTime: 0,
        }
    }

    #[test]
    fn no_system_battery_is_not_present() {
        let s = status(1, 0x80, 0xFF, u32::MAX);
        let data = battery_data_from_status(&s);
        assert!(!data.is_present);
    }

    #[test]
    fn discharging_on_battery() {
        // High charge (>66%), not charging, not on AC, 1h30m remaining.
        let s = status(0, 0x01, 72, 5400);
        let data = battery_data_from_status(&s);
        assert!(data.is_present);
        assert_eq!(data.percent, 72.0);
        assert!(!data.ac_online);
        assert!(!data.charging);
        assert_eq!(data.seconds_remaining, Some(5400));
    }

    #[test]
    fn charging_on_ac() {
        let s = status(1, 0x08, 45, u32::MAX);
        let data = battery_data_from_status(&s);
        assert!(data.is_present);
        assert!(data.ac_online);
        assert!(data.charging);
        assert_eq!(data.seconds_remaining, None);
    }

    #[test]
    fn fully_unknown_status_is_treated_as_not_present() {
        // BatteryFlag 0xFF ("unknown status") happens to also carry the
        // NO_SYSTEM_BATTERY (0x80) bit, so it takes the same safe path as a
        // real no-battery reading rather than fabricating a 0% reading for
        // a battery that might actually exist.
        let s = status(255, 0xFF, 0xFF, u32::MAX);
        let data = battery_data_from_status(&s);
        assert!(!data.is_present);
    }
}
