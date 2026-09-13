use std::collections::HashMap;
use std::mem;
use std::time::Instant;
use windows_sys::Win32::Foundation::CloseHandle;
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows_sys::Win32::System::ProcessStatus::{K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
use windows_sys::Win32::System::Threading::{
    GetProcessTimes, OpenProcess, TerminateProcess, PROCESS_QUERY_LIMITED_INFORMATION,
    PROCESS_TERMINATE,
};
use crate::config::ProcessSortBy;
use crate::model::ProcessItem;

pub struct ProcessCollector {
    prev_times: HashMap<u32, (u64, Instant)>,
    num_cores: usize,
}

impl ProcessCollector {
    pub fn new(num_cores: usize) -> Self {
        Self {
            prev_times: HashMap::new(),
            num_cores: num_cores.max(1),
        }
    }

    pub fn collect(&mut self, sort_by: ProcessSortBy, desc: bool, filter: &str) -> Vec<ProcessItem> {
        let now = Instant::now();
        let mut processes = Vec::with_capacity(256);
        let mut seen_pids = HashMap::new();

        unsafe {
            let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
            if snapshot.is_null() || snapshot == -1isize as _ {
                return processes;
            }

            let mut entry: PROCESSENTRY32W = mem::zeroed();
            entry.dwSize = mem::size_of::<PROCESSENTRY32W>() as u32;

            if Process32FirstW(snapshot, &mut entry) != 0 {
                loop {
                    let pid = entry.th32ProcessID;
                    let name_len = entry.szExeFile.iter().position(|&c| c == 0).unwrap_or(entry.szExeFile.len());
                    let name = String::from_utf16_lossy(&entry.szExeFile[..name_len]);

                    // Skip System Idle Process (PID 0)
                    if pid != 0 {
                        let mut mem_bytes = 0u64;
                        let mut cpu_pct = 0.0f32;

                        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
                        if !handle.is_null() {
                            let mut pmc: PROCESS_MEMORY_COUNTERS = mem::zeroed();
                            pmc.cb = mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
                            if K32GetProcessMemoryInfo(handle, &mut pmc, pmc.cb) != 0 {
                                mem_bytes = pmc.WorkingSetSize as u64;
                            }

                            let mut creation = mem::zeroed();
                            let mut exit = mem::zeroed();
                            let mut kernel = mem::zeroed();
                            let mut user = mem::zeroed();
                            if GetProcessTimes(handle, &mut creation, &mut exit, &mut kernel, &mut user) != 0 {
                                let k_time = (kernel.dwLowDateTime as u64) | ((kernel.dwHighDateTime as u64) << 32);
                                let u_time = (user.dwLowDateTime as u64) | ((user.dwHighDateTime as u64) << 32);
                                let total_time_100ns = k_time + u_time;

                                if let Some(&(prev_time_100ns, prev_instant)) = self.prev_times.get(&pid) {
                                    let dt = now.duration_since(prev_instant).as_secs_f64();
                                    if dt > 0.05 && total_time_100ns >= prev_time_100ns {
                                        let delta_proc_100ns = total_time_100ns - prev_time_100ns;
                                        let delta_wall_100ns = dt * 10_000_000.0;
                                        let raw_pct = (delta_proc_100ns as f64 / delta_wall_100ns) * 100.0 / (self.num_cores as f64);
                                        cpu_pct = raw_pct.clamp(0.0, 100.0) as f32;
                                    }
                                }

                                seen_pids.insert(pid, (total_time_100ns, now));
                            }

                            CloseHandle(handle);
                        }

                        let match_filter = if filter.is_empty() {
                            true
                        } else {
                            name.to_lowercase().contains(&filter.to_lowercase())
                                || pid.to_string().contains(filter)
                        };

                        if match_filter {
                            processes.push(ProcessItem {
                                pid,
                                name,
                                cpu_pct,
                                mem_bytes,
                                threads: entry.cntThreads,
                            });
                        }
                    }

                    if Process32NextW(snapshot, &mut entry) == 0 {
                        break;
                    }
                }
            }
            CloseHandle(snapshot);
        }

        // Retain only currently living processes in prev_times to prevent memory leaks
        self.prev_times = seen_pids;

        // Sort processes
        match sort_by {
            ProcessSortBy::Cpu => {
                processes.sort_by(|a, b| {
                    if desc {
                        b.cpu_pct.partial_cmp(&a.cpu_pct).unwrap_or(std::cmp::Ordering::Equal)
                    } else {
                        a.cpu_pct.partial_cmp(&b.cpu_pct).unwrap_or(std::cmp::Ordering::Equal)
                    }
                });
            }
            ProcessSortBy::Memory => {
                processes.sort_by(|a, b| {
                    if desc {
                        b.mem_bytes.cmp(&a.mem_bytes)
                    } else {
                        a.mem_bytes.cmp(&b.mem_bytes)
                    }
                });
            }
            ProcessSortBy::Pid => {
                processes.sort_by(|a, b| {
                    if desc {
                        b.pid.cmp(&a.pid)
                    } else {
                        a.pid.cmp(&b.pid)
                    }
                });
            }
            ProcessSortBy::Name => {
                processes.sort_by(|a, b| {
                    if desc {
                        b.name.to_lowercase().cmp(&a.name.to_lowercase())
                    } else {
                        a.name.to_lowercase().cmp(&b.name.to_lowercase())
                    }
                });
            }
        }

        processes
    }

    pub fn kill_process(pid: u32) -> Result<(), String> {
        unsafe {
            let handle = OpenProcess(PROCESS_TERMINATE, 0, pid);
            if handle.is_null() {
                return Err(format!("Access denied or invalid handle for PID {}", pid));
            }
            let success = TerminateProcess(handle, 1);
            CloseHandle(handle);
            if success != 0 {
                Ok(())
            } else {
                Err(format!("Failed to terminate process PID {}", pid))
            }
        }
    }
}
