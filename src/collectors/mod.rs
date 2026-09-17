pub mod battery;
pub mod cpu;
pub mod gpu;
pub mod io;
pub mod memory;
pub mod network;
pub mod process;

use self::battery::BatteryCollector;
use self::cpu::CpuCollector;
use self::gpu::GpuCollector;
use self::io::IoCollector;
use self::memory::MemoryCollector;
use self::network::NetworkCollector;
use self::process::ProcessCollector;
use crate::config::AppConfig;
use crate::model::SystemSnapshot;
use std::time::Instant;

pub struct SystemCollector {
    cpu: CpuCollector,
    memory: MemoryCollector,
    io: IoCollector,
    gpu: GpuCollector,
    net: NetworkCollector,
    battery: BatteryCollector,
    proc: ProcessCollector,
}

impl SystemCollector {
    pub fn new() -> Self {
        let history_len = 160;
        let cpu = CpuCollector::new(history_len);
        let num_cores = num_cpus();

        Self {
            cpu,
            memory: MemoryCollector::new(history_len),
            io: IoCollector::new(history_len),
            gpu: GpuCollector::new(history_len),
            net: NetworkCollector::new(history_len),
            battery: BatteryCollector::new(history_len),
            proc: ProcessCollector::new(num_cores),
        }
    }

    pub fn collect(&mut self, config: &AppConfig, filter: &str) -> SystemSnapshot {
        let timestamp = Instant::now();
        let cpu_data = self.cpu.collect();
        let (mem_data, uptime_secs) = self.memory.collect();
        let io_data = self.io.collect();
        let gpu_data = self.gpu.collect();
        let net_data = self.net.collect();
        let battery_data = self.battery.collect();
        let processes = self
            .proc
            .collect(config.proc_sort_by, config.proc_sort_desc, filter);

        SystemSnapshot {
            timestamp,
            uptime_secs,
            cpu: cpu_data,
            memory: mem_data,
            io: io_data,
            gpu: gpu_data,
            net: net_data,
            battery: battery_data,
            processes,
        }
    }
}

fn num_cpus() -> usize {
    #[repr(C)]
    struct SystemInfo {
        w_processor_architecture: u16,
        w_reserved: u16,
        dw_page_size: u32,
        lp_minimum_application_address: *mut std::ffi::c_void,
        lp_maximum_application_address: *mut std::ffi::c_void,
        dw_active_processor_mask: usize,
        dw_number_of_processors: u32,
        dw_processor_type: u32,
        dw_allocation_granularity: u32,
        w_processor_level: u16,
        w_processor_revision: u16,
    }

    unsafe extern "system" {
        fn GetSystemInfo(lp_system_info: *mut SystemInfo);
    }

    unsafe {
        let mut info: SystemInfo = std::mem::zeroed();
        GetSystemInfo(&mut info);
        info.dw_number_of_processors.max(1) as usize
    }
}
