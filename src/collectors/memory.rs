use std::mem;
use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX, GetTickCount64};
use crate::model::MemoryData;

pub struct MemoryCollector {
    history: Vec<f64>,
    max_history: usize,
}

impl MemoryCollector {
    pub fn new(max_history: usize) -> Self {
        Self {
            history: Vec::new(),
            max_history,
        }
    }

    pub fn collect(&mut self) -> (MemoryData, u64) {
        let mut mem_status: MEMORYSTATUSEX = unsafe { mem::zeroed() };
        mem_status.dwLength = mem::size_of::<MEMORYSTATUSEX>() as u32;

        let ok = unsafe { GlobalMemoryStatusEx(&mut mem_status) };
        let uptime_secs = unsafe { GetTickCount64() / 1000 };

        if ok != 0 {
            let total_bytes = mem_status.ullTotalPhys;
            let free_bytes = mem_status.ullAvailPhys;
            let used_bytes = total_bytes.saturating_sub(free_bytes);
            let usage_pct = if total_bytes > 0 {
                (used_bytes as f64 / total_bytes as f64) * 100.0
            } else {
                0.0
            };

            let commit_total_bytes = mem_status.ullTotalPageFile;
            let commit_free_bytes = mem_status.ullAvailPageFile;
            let commit_used_bytes = commit_total_bytes.saturating_sub(commit_free_bytes);
            let commit_pct = if commit_total_bytes > 0 {
                (commit_used_bytes as f64 / commit_total_bytes as f64) * 100.0
            } else {
                0.0
            };

            self.history.push(usage_pct);
            if self.history.len() > self.max_history {
                self.history.remove(0);
            }

            (
                MemoryData {
                    total_bytes,
                    used_bytes,
                    free_bytes,
                    usage_pct,
                    commit_total_bytes,
                    commit_used_bytes,
                    commit_pct,
                    history: self.history.clone(),
                },
                uptime_secs,
            )
        } else {
            (MemoryData::default(), uptime_secs)
        }
    }
}
