use log::trace;
use std::fs;
use sysinfo::System;

pub mod cpu;
pub mod ip;
pub mod load;
pub mod mem;
pub mod network;
pub mod os;

pub fn realtime_uptime() -> u64 {
    let uptime = System::uptime();
    trace!("REALTIME UPTIME successfully retrieved: {uptime}");
    uptime
}

pub fn realtime_process() -> u64 {
    let mut process_count = 0;

    let Ok(entries) = fs::read_dir("/proc") else {
        trace!("REALTIME PROCESS failed: Cannot read /proc directory");
        return 0;
    };

    for entry in entries.flatten() {
        let file_name = entry.file_name();
        if let Some(name_str) = file_name.to_str()
            && name_str.parse::<u32>().is_ok()
        {
            process_count += 1;
        }
    }

    let process_count = process_count as u64;
    trace!("REALTIME PROCESS successfully retrieved: {process_count}");
    process_count
}
