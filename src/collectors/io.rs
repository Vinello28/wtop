use crate::model::{DiskIoData, DiskPartition};
use std::mem;
use windows_sys::Win32::Storage::FileSystem::{
    GetDiskFreeSpaceExW, GetDriveTypeW, GetLogicalDriveStringsW,
};
use windows_sys::Win32::System::Performance::{
    PDH_FMT_COUNTERVALUE, PDH_FMT_DOUBLE, PdhAddEnglishCounterW, PdhCloseQuery,
    PdhCollectQueryData, PdhGetFormattedCounterValue, PdhOpenQueryW,
};

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

pub struct IoCollector {
    pdh_query: isize,
    h_read: isize,
    h_write: isize,
    read_history: Vec<f64>,
    write_history: Vec<f64>,
    max_history: usize,
}

impl IoCollector {
    pub fn new(max_history: usize) -> Self {
        let mut pdh_query = 0;
        let mut h_read = 0;
        let mut h_write = 0;

        unsafe {
            let res = PdhOpenQueryW(std::ptr::null(), 0, &mut pdh_query);
            if res == 0 {
                let r_path = to_wide(r"\PhysicalDisk(_Total)\Disk Read Bytes/sec");
                let w_path = to_wide(r"\PhysicalDisk(_Total)\Disk Write Bytes/sec");

                let _ = PdhAddEnglishCounterW(pdh_query, r_path.as_ptr(), 0, &mut h_read);
                let _ = PdhAddEnglishCounterW(pdh_query, w_path.as_ptr(), 0, &mut h_write);
                PdhCollectQueryData(pdh_query);
            }
        }

        Self {
            pdh_query,
            h_read,
            h_write,
            read_history: Vec::new(),
            write_history: Vec::new(),
            max_history,
        }
    }

    pub fn collect(&mut self) -> DiskIoData {
        let mut read_bytes_sec = 0.0;
        let mut write_bytes_sec = 0.0;

        if self.pdh_query != 0 {
            unsafe {
                if PdhCollectQueryData(self.pdh_query) == 0 {
                    if self.h_read != 0 {
                        let mut val: PDH_FMT_COUNTERVALUE = mem::zeroed();
                        if PdhGetFormattedCounterValue(
                            self.h_read,
                            PDH_FMT_DOUBLE,
                            std::ptr::null_mut(),
                            &mut val,
                        ) == 0
                        {
                            read_bytes_sec = val.Anonymous.doubleValue.max(0.0);
                        }
                    }
                    if self.h_write != 0 {
                        let mut val: PDH_FMT_COUNTERVALUE = mem::zeroed();
                        if PdhGetFormattedCounterValue(
                            self.h_write,
                            PDH_FMT_DOUBLE,
                            std::ptr::null_mut(),
                            &mut val,
                        ) == 0
                        {
                            write_bytes_sec = val.Anonymous.doubleValue.max(0.0);
                        }
                    }
                }
            }
        }

        self.read_history.push(read_bytes_sec);
        if self.read_history.len() > self.max_history {
            self.read_history.remove(0);
        }

        self.write_history.push(write_bytes_sec);
        if self.write_history.len() > self.max_history {
            self.write_history.remove(0);
        }

        let partitions = self.get_partitions();

        DiskIoData {
            read_bytes_sec,
            write_bytes_sec,
            read_history: self.read_history.clone(),
            write_history: self.write_history.clone(),
            partitions,
        }
    }

    fn get_partitions(&self) -> Vec<DiskPartition> {
        let mut partitions = Vec::new();
        let mut buffer = [0u16; 512];
        let len = unsafe { GetLogicalDriveStringsW(buffer.len() as u32, buffer.as_mut_ptr()) };

        if len > 0 {
            let mut start = 0;
            for i in 0..len as usize {
                if buffer[i] == 0 {
                    if start < i {
                        let drive_slice = &buffer[start..i];
                        let drive_str = String::from_utf16_lossy(drive_slice);
                        let mut wide_drive = drive_slice.to_vec();
                        wide_drive.push(0);

                        let drive_type = unsafe { GetDriveTypeW(wide_drive.as_ptr()) };
                        // 3 = DRIVE_FIXED, 2 = DRIVE_REMOVABLE
                        if drive_type == 3 || drive_type == 2 {
                            let mut free_bytes_avail = 0u64;
                            let mut total_bytes = 0u64;
                            let mut total_free_bytes = 0u64;

                            let ok = unsafe {
                                GetDiskFreeSpaceExW(
                                    wide_drive.as_ptr(),
                                    &mut free_bytes_avail,
                                    &mut total_bytes,
                                    &mut total_free_bytes,
                                )
                            };

                            if ok != 0 && total_bytes > 0 {
                                let used_bytes = total_bytes.saturating_sub(total_free_bytes);
                                let usage_pct = (used_bytes as f64 / total_bytes as f64) * 100.0;
                                partitions.push(DiskPartition {
                                    mount: drive_str,
                                    total_bytes,
                                    free_bytes: total_free_bytes,
                                    used_bytes,
                                    usage_pct,
                                });
                            }
                        }
                    }
                    start = i + 1;
                }
            }
        }

        partitions
    }
}

impl Drop for IoCollector {
    fn drop(&mut self) {
        if self.pdh_query != 0 {
            unsafe {
                PdhCloseQuery(self.pdh_query);
            }
        }
    }
}
