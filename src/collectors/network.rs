use std::time::Instant;
use windows_sys::Win32::NetworkManagement::IpHelper::{FreeMibTable, GetIfTable2, MIB_IF_TABLE2};
use crate::model::NetworkData;

pub struct NetworkCollector {
    prev_rx_total: u64,
    prev_tx_total: u64,
    prev_time: Option<Instant>,
    rx_history: Vec<f64>,
    tx_history: Vec<f64>,
    max_history: usize,
    active_iface: String,
}

impl NetworkCollector {
    pub fn new(max_history: usize) -> Self {
        Self {
            prev_rx_total: 0,
            prev_tx_total: 0,
            prev_time: None,
            rx_history: Vec::new(),
            tx_history: Vec::new(),
            max_history,
            active_iface: "Primary Adapter".to_string(),
        }
    }

    pub fn collect(&mut self) -> NetworkData {
        let now = Instant::now();
        let mut curr_rx_total = 0u64;
        let mut curr_tx_total = 0u64;
        let mut best_iface = String::new();
        let mut max_traffic = 0u64;

        unsafe {
            let mut table: *mut MIB_IF_TABLE2 = std::ptr::null_mut();
            if GetIfTable2(&mut table) == 0 && !table.is_null() {
                let num = (*table).NumEntries as usize;
                let rows = std::slice::from_raw_parts((*table).Table.as_ptr(), num);

                for row in rows {
                    // Filter: OperStatus == 1 (IfOperStatusUp), Type != 24 (loopback)
                    if row.OperStatus == 1 && row.Type != 24 {
                        let rx = row.InOctets;
                        let tx = row.OutOctets;
                        curr_rx_total += rx;
                        curr_tx_total += tx;

                        let sum = rx + tx;
                        if sum > max_traffic {
                            max_traffic = sum;
                            let desc_len = row.Description.iter().position(|&c| c == 0).unwrap_or(row.Description.len());
                            let desc = String::from_utf16_lossy(&row.Description[..desc_len]).trim().to_string();
                            if !desc.is_empty() {
                                best_iface = desc;
                            }
                        }
                    }
                }
                FreeMibTable(table as *const _);
            }
        }

        if !best_iface.is_empty() {
            self.active_iface = best_iface;
        }

        let mut rx_speed = 0.0;
        let mut tx_speed = 0.0;

        if let Some(prev_t) = self.prev_time {
            let dt = now.duration_since(prev_t).as_secs_f64();
            if dt > 0.05 && self.prev_rx_total > 0 && curr_rx_total >= self.prev_rx_total {
                let d_rx = curr_rx_total - self.prev_rx_total;
                let d_tx = curr_tx_total - self.prev_tx_total;
                rx_speed = d_rx as f64 / dt;
                tx_speed = d_tx as f64 / dt;
            }
        }

        self.prev_time = Some(now);
        self.prev_rx_total = curr_rx_total;
        self.prev_tx_total = curr_tx_total;

        self.rx_history.push(rx_speed);
        if self.rx_history.len() > self.max_history {
            self.rx_history.remove(0);
        }

        self.tx_history.push(tx_speed);
        if self.tx_history.len() > self.max_history {
            self.tx_history.remove(0);
        }

        NetworkData {
            rx_bytes_sec: rx_speed,
            tx_bytes_sec: tx_speed,
            rx_total_bytes: curr_rx_total,
            tx_total_bytes: curr_tx_total,
            rx_history: self.rx_history.clone(),
            tx_history: self.tx_history.clone(),
            active_iface: self.active_iface.clone(),
        }
    }
}
