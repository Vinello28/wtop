use std::mem;
use crate::model::{CpuCore, CpuData};

#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
struct SystemProcessorPerformanceInfo {
    idle_time: i64,
    kernel_time: i64,
    user_time: i64,
    dpc_time: i64,
    interrupt_time: i64,
    interrupt_count: u32,
}

type NtQuerySystemInformationFn = unsafe extern "system" fn(
    system_information_class: u32,
    system_information: *mut std::ffi::c_void,
    system_information_length: u32,
    return_length: *mut u32,
) -> i32;

pub struct CpuCollector {
    prev_cores: Vec<SystemProcessorPerformanceInfo>,
    model_name: String,
    history: Vec<f64>,
    max_history: usize,
    query_fn: Option<NtQuerySystemInformationFn>,
}

impl CpuCollector {
    pub fn new(max_history: usize) -> Self {
        let model_name = get_cpu_model_name();
        let query_fn = unsafe {
            let ntdll = windows_sys::Win32::System::LibraryLoader::LoadLibraryA(b"ntdll.dll\0".as_ptr());
            if !ntdll.is_null() {
                let proc = windows_sys::Win32::System::LibraryLoader::GetProcAddress(
                    ntdll,
                    b"NtQuerySystemInformation\0".as_ptr(),
                );
                proc.map(|p| mem::transmute(p))
            } else {
                None
            }
        };

        let mut collector = Self {
            prev_cores: Vec::new(),
            model_name,
            history: Vec::new(),
            max_history,
            query_fn,
        };

        // Take initial baseline sample
        let _ = collector.query_raw_cores();
        collector
    }

    fn query_raw_cores(&mut self) -> Option<Vec<SystemProcessorPerformanceInfo>> {
        let func = self.query_fn?;
        let mut buf = vec![SystemProcessorPerformanceInfo::default(); 128];
        let mut ret_len = 0u32;
        let status = unsafe {
            func(
                8, // SystemProcessorPerformanceInformation
                buf.as_mut_ptr() as *mut _,
                (buf.len() * mem::size_of::<SystemProcessorPerformanceInfo>()) as u32,
                &mut ret_len,
            )
        };

        if status == 0 && ret_len > 0 {
            let num_cores = ret_len as usize / mem::size_of::<SystemProcessorPerformanceInfo>();
            buf.truncate(num_cores);
            Some(buf)
        } else {
            None
        }
    }

    pub fn collect(&mut self) -> CpuData {
        let raw_cores = self.query_raw_cores();
        let mut cores = Vec::new();
        let mut global_pct = 0.0;

        if let Some(current) = raw_cores {
            if !self.prev_cores.is_empty() && self.prev_cores.len() == current.len() {
                let mut total_system = 0i64;
                let mut total_idle = 0i64;

                for (i, (curr, prev)) in current.iter().zip(self.prev_cores.iter()).enumerate() {
                    // On Windows, kernel_time includes idle_time!
                    let kernel = curr.kernel_time - prev.kernel_time;
                    let user = curr.user_time - prev.user_time;
                    let idle = curr.idle_time - prev.idle_time;

                    let total = kernel + user;
                    let usage = if total > 0 {
                        let active = total - idle;
                        (active as f64 / total as f64) * 100.0
                    } else {
                        0.0
                    };
                    let clamped = usage.clamp(0.0, 100.0);

                    total_system += total;
                    total_idle += idle;

                    cores.push(CpuCore {
                        id: i,
                        usage_pct: clamped,
                    });
                }

                if total_system > 0 {
                    let active = total_system - total_idle;
                    global_pct = ((active as f64 / total_system as f64) * 100.0).clamp(0.0, 100.0);
                }
            } else {
                for i in 0..current.len() {
                    cores.push(CpuCore { id: i, usage_pct: 0.0 });
                }
            }
            self.prev_cores = current;
        }

        self.history.push(global_pct);
        if self.history.len() > self.max_history {
            self.history.remove(0);
        }

        CpuData {
            global_pct,
            cores,
            history: self.history.clone(),
            model_name: self.model_name.clone(),
        }
    }
}

fn get_cpu_model_name() -> String {
    #[cfg(target_arch = "x86_64")]
    {
        let mut brand_bytes = [0u8; 48];
        for (i, &ext) in [0x80000002u32, 0x80000003, 0x80000004].iter().enumerate() {
            let res = std::arch::x86_64::__cpuid(ext);
            let offset = i * 16;
            brand_bytes[offset..offset + 4].copy_from_slice(&res.eax.to_le_bytes());
            brand_bytes[offset + 4..offset + 8].copy_from_slice(&res.ebx.to_le_bytes());
            brand_bytes[offset + 8..offset + 12].copy_from_slice(&res.ecx.to_le_bytes());
            brand_bytes[offset + 12..offset + 16].copy_from_slice(&res.edx.to_le_bytes());
        }

        if let Ok(brand) = std::str::from_utf8(&brand_bytes) {
            let trimmed = brand.trim_matches(char::from(0)).trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }
    }

    "Windows Processor".to_string()
}
