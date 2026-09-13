use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemeMode {
    Dark,
    Light,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProcessSortBy {
    Cpu,
    Memory,
    Pid,
    Name,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub sampling_rate_ms: u64,
    pub theme: ThemeMode,
    pub proc_sort_by: ProcessSortBy,
    pub proc_sort_desc: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            sampling_rate_ms: 1000,
            theme: ThemeMode::Dark,
            proc_sort_by: ProcessSortBy::Cpu,
            proc_sort_desc: true,
        }
    }
}

impl AppConfig {
    fn config_path() -> Option<PathBuf> {
        let appdata = std::env::var_os("APPDATA")?;
        let mut path = PathBuf::from(appdata);
        path.push("wtop");
        let _ = fs::create_dir_all(&path);
        path.push("config.json");
        Some(path)
    }

    pub fn load() -> Self {
        if let Some(path) = Self::config_path() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(cfg) = serde_json::from_str::<AppConfig>(&content) {
                    return cfg;
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) {
        if let Some(path) = Self::config_path() {
            if let Ok(content) = serde_json::to_string_pretty(self) {
                let _ = fs::write(path, content);
            }
        }
    }

    pub fn cycle_sampling_rate(&mut self, increase: bool) {
        let rates = [250, 500, 1000, 1500, 2000, 3000, 5000];
        let current = self.sampling_rate_ms;
        if increase {
            // slower sampling = higher interval
            if let Some(&next) = rates.iter().find(|&&r| r > current) {
                self.sampling_rate_ms = next;
            }
        } else {
            // faster sampling = lower interval
            if let Some(&prev) = rates.iter().rev().find(|&&r| r < current) {
                self.sampling_rate_ms = prev;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sampling_rate_cycle() {
        let mut cfg = AppConfig {
            sampling_rate_ms: 1000,
            ..Default::default()
        };

        // Faster (lower interval)
        cfg.cycle_sampling_rate(false);
        assert_eq!(cfg.sampling_rate_ms, 500);

        cfg.cycle_sampling_rate(false);
        assert_eq!(cfg.sampling_rate_ms, 250);

        // Cannot go below 250
        cfg.cycle_sampling_rate(false);
        assert_eq!(cfg.sampling_rate_ms, 250);

        // Slower (higher interval)
        cfg.cycle_sampling_rate(true);
        assert_eq!(cfg.sampling_rate_ms, 500);
    }

    #[test]
    fn test_config_serde() {
        let cfg = AppConfig::default();
        let json = serde_json::to_string(&cfg).expect("Serialize config");
        let deserialized: AppConfig = serde_json::from_str(&json).expect("Deserialize config");
        assert_eq!(deserialized.sampling_rate_ms, 1000);
        assert_eq!(deserialized.theme, ThemeMode::Dark);
    }
}
