use crate::model::GpuData;
use std::mem;
use windows_sys::Win32::System::Performance::{
    PDH_FMT_COUNTERVALUE_ITEM_W, PDH_FMT_DOUBLE, PdhAddEnglishCounterW, PdhCloseQuery,
    PdhCollectQueryData, PdhGetFormattedCounterArrayW, PdhOpenQueryW,
};

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

pub struct GpuCollector {
    pdh_query: isize,
    h_engine: isize,
    h_vram: isize,
    name: String,
    history: Vec<f64>,
    max_history: usize,
    is_available: bool,
}

impl GpuCollector {
    pub fn new(max_history: usize) -> Self {
        let name = get_gpu_adapter_name();
        let mut pdh_query = 0;
        let mut h_engine = 0;
        let mut h_vram = 0;
        let mut is_available = false;

        unsafe {
            let res = PdhOpenQueryW(std::ptr::null(), 0, &mut pdh_query);
            if res == 0 {
                let engine_path = to_wide(r"\GPU Engine(*)\Utilization Percentage");
                let vram_path = to_wide(r"\GPU Adapter Memory(*)\Dedicated Usage");

                let r1 = PdhAddEnglishCounterW(pdh_query, engine_path.as_ptr(), 0, &mut h_engine);
                let _r2 = PdhAddEnglishCounterW(pdh_query, vram_path.as_ptr(), 0, &mut h_vram);

                if r1 == 0 {
                    is_available = true;
                    PdhCollectQueryData(pdh_query);
                }
            }
        }

        Self {
            pdh_query,
            h_engine,
            h_vram,
            name,
            history: Vec::new(),
            max_history,
            is_available,
        }
    }

    pub fn collect(&mut self) -> GpuData {
        if !self.is_available || self.pdh_query == 0 {
            return GpuData {
                name: self.name.clone(),
                utilization_pct: 0.0,
                history: self.history.clone(),
                dedicated_vram_used: 0,
                dedicated_vram_total: 0,
                vram_pct: 0.0,
                is_available: false,
            };
        }

        let mut utilization_pct = 0.0;
        let mut dedicated_vram_used = 0u64;

        unsafe {
            if PdhCollectQueryData(self.pdh_query) == 0 {
                // Collect GPU engine utilization
                if self.h_engine != 0 {
                    let mut buffer_size = 0u32;
                    let mut item_count = 0u32;
                    let _ = PdhGetFormattedCounterArrayW(
                        self.h_engine,
                        PDH_FMT_DOUBLE,
                        &mut buffer_size,
                        &mut item_count,
                        std::ptr::null_mut(),
                    );

                    if buffer_size > 0 {
                        let mut buf = vec![0u8; buffer_size as usize];
                        if PdhGetFormattedCounterArrayW(
                            self.h_engine,
                            PDH_FMT_DOUBLE,
                            &mut buffer_size,
                            &mut item_count,
                            buf.as_mut_ptr() as *mut PDH_FMT_COUNTERVALUE_ITEM_W,
                        ) == 0
                        {
                            let items = std::slice::from_raw_parts(
                                buf.as_ptr() as *const PDH_FMT_COUNTERVALUE_ITEM_W,
                                item_count as usize,
                            );
                            let mut sum = 0.0;
                            for item in items {
                                let val = item.FmtValue.Anonymous.doubleValue;
                                if val > 0.0 {
                                    sum += val;
                                }
                            }
                            utilization_pct = sum.clamp(0.0, 100.0);
                        }
                    }
                }

                // Collect VRAM
                if self.h_vram != 0 {
                    let mut buffer_size = 0u32;
                    let mut item_count = 0u32;
                    let _ = PdhGetFormattedCounterArrayW(
                        self.h_vram,
                        PDH_FMT_DOUBLE,
                        &mut buffer_size,
                        &mut item_count,
                        std::ptr::null_mut(),
                    );

                    if buffer_size > 0 {
                        let mut buf = vec![0u8; buffer_size as usize];
                        if PdhGetFormattedCounterArrayW(
                            self.h_vram,
                            PDH_FMT_DOUBLE,
                            &mut buffer_size,
                            &mut item_count,
                            buf.as_mut_ptr() as *mut PDH_FMT_COUNTERVALUE_ITEM_W,
                        ) == 0
                        {
                            let items = std::slice::from_raw_parts(
                                buf.as_ptr() as *const PDH_FMT_COUNTERVALUE_ITEM_W,
                                item_count as usize,
                            );
                            let mut sum_vram = 0.0;
                            for item in items {
                                let val = item.FmtValue.Anonymous.doubleValue;
                                if val > 0.0 {
                                    sum_vram += val;
                                }
                            }
                            dedicated_vram_used = sum_vram as u64;
                        }
                    }
                }
            }
        }

        self.history.push(utilization_pct);
        if self.history.len() > self.max_history {
            self.history.remove(0);
        }

        GpuData {
            name: self.name.clone(),
            utilization_pct,
            history: self.history.clone(),
            dedicated_vram_used,
            dedicated_vram_total: 0,
            vram_pct: 0.0,
            is_available: true,
        }
    }
}

impl Drop for GpuCollector {
    fn drop(&mut self) {
        if self.pdh_query != 0 {
            unsafe {
                PdhCloseQuery(self.pdh_query);
            }
        }
    }
}

fn get_gpu_adapter_name() -> String {
    // Attempt to get device string via user32 EnumDisplayDevicesW
    #[repr(C)]
    struct DisplayDeviceW {
        cb: u32,
        device_name: [u16; 32],
        device_string: [u16; 128],
        state_flags: u32,
        device_id: [u16; 128],
        device_key: [u16; 128],
    }

    type EnumDisplayDevicesWFn = unsafe extern "system" fn(
        lp_device: *const u16,
        i_dev_num: u32,
        lp_display_device: *mut DisplayDeviceW,
        dw_flags: u32,
    ) -> i32;

    unsafe {
        let user32 =
            windows_sys::Win32::System::LibraryLoader::LoadLibraryA(c"user32.dll".as_ptr().cast());
        if !user32.is_null() {
            let proc = windows_sys::Win32::System::LibraryLoader::GetProcAddress(
                user32,
                c"EnumDisplayDevicesW".as_ptr().cast(),
            );
            if let Some(func) = proc {
                let enum_fn: EnumDisplayDevicesWFn = mem::transmute(func);
                let mut dev: DisplayDeviceW = mem::zeroed();
                dev.cb = mem::size_of::<DisplayDeviceW>() as u32;
                if enum_fn(std::ptr::null(), 0, &mut dev, 0) != 0 {
                    let len = dev
                        .device_string
                        .iter()
                        .position(|&c| c == 0)
                        .unwrap_or(dev.device_string.len());
                    let name = String::from_utf16_lossy(&dev.device_string[..len])
                        .trim()
                        .to_string();
                    if !name.is_empty() {
                        return name;
                    }
                }
            }
        }
    }

    "GPU (WDDM Adapter)".to_string()
}
