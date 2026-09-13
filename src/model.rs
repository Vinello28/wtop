#![allow(dead_code)]

use std::time::Instant;

#[derive(Debug, Clone)]
pub struct CpuCore {
    pub id: usize,
    pub usage_pct: f64,
}

#[derive(Debug, Clone, Default)]
pub struct CpuData {
    pub global_pct: f64,
    pub cores: Vec<CpuCore>,
    pub history: Vec<f64>,
    pub model_name: String,
}

#[derive(Debug, Clone, Default)]
pub struct MemoryData {
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub free_bytes: u64,
    pub usage_pct: f64,
    pub commit_total_bytes: u64,
    pub commit_used_bytes: u64,
    pub commit_pct: f64,
    pub history: Vec<f64>,
}

#[derive(Debug, Clone)]
pub struct DiskPartition {
    pub mount: String,
    pub total_bytes: u64,
    pub free_bytes: u64,
    pub used_bytes: u64,
    pub usage_pct: f64,
}

#[derive(Debug, Clone, Default)]
pub struct DiskIoData {
    pub read_bytes_sec: f64,
    pub write_bytes_sec: f64,
    pub read_history: Vec<f64>,
    pub write_history: Vec<f64>,
    pub partitions: Vec<DiskPartition>,
}

#[derive(Debug, Clone, Default)]
pub struct GpuData {
    pub name: String,
    pub utilization_pct: f64,
    pub history: Vec<f64>,
    pub dedicated_vram_used: u64,
    pub dedicated_vram_total: u64,
    pub vram_pct: f64,
    pub is_available: bool,
}

#[derive(Debug, Clone, Default)]
pub struct NetworkData {
    pub rx_bytes_sec: f64,
    pub tx_bytes_sec: f64,
    pub rx_total_bytes: u64,
    pub tx_total_bytes: u64,
    pub rx_history: Vec<f64>,
    pub tx_history: Vec<f64>,
    pub active_iface: String,
}

#[derive(Debug, Clone)]
pub struct ProcessItem {
    pub pid: u32,
    pub name: String,
    pub cpu_pct: f32,
    pub mem_bytes: u64,
    pub threads: u32,
}

#[derive(Debug, Clone)]
pub struct SystemSnapshot {
    pub timestamp: Instant,
    pub uptime_secs: u64,
    pub cpu: CpuData,
    pub memory: MemoryData,
    pub io: DiskIoData,
    pub gpu: GpuData,
    pub net: NetworkData,
    pub processes: Vec<ProcessItem>,
}

impl Default for SystemSnapshot {
    fn default() -> Self {
        Self {
            timestamp: Instant::now(),
            uptime_secs: 0,
            cpu: CpuData::default(),
            memory: MemoryData::default(),
            io: DiskIoData::default(),
            gpu: GpuData::default(),
            net: NetworkData::default(),
            processes: Vec::new(),
        }
    }
}
