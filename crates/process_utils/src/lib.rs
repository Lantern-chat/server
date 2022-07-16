#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Priority {
    Idle,
    BelowNormal,
    Normal,
    AboveNormal,
    High,
    Realtime,
}

#[cfg(not(any(windows, unix, target_os = "macos")))]
pub fn set_own_process_priority(priority: Priority) -> bool {
    compile_error!("Here");
    false
}

#[cfg(windows)]
pub fn set_own_process_priority(priority: Priority) -> bool {
    use windows::Win32::System::Threading::{GetCurrentProcess, SetPriorityClass};

    use windows::Win32::System::Threading::{
        ABOVE_NORMAL_PRIORITY_CLASS, BELOW_NORMAL_PRIORITY_CLASS, HIGH_PRIORITY_CLASS, IDLE_PRIORITY_CLASS,
        NORMAL_PRIORITY_CLASS, REALTIME_PRIORITY_CLASS,
    };

    unsafe {
        SetPriorityClass(
            GetCurrentProcess(),
            match priority {
                Priority::Idle => IDLE_PRIORITY_CLASS,
                Priority::BelowNormal => BELOW_NORMAL_PRIORITY_CLASS,
                Priority::Normal => NORMAL_PRIORITY_CLASS,
                Priority::AboveNormal => ABOVE_NORMAL_PRIORITY_CLASS,
                Priority::High => HIGH_PRIORITY_CLASS,
                Priority::Realtime => REALTIME_PRIORITY_CLASS,
            },
        )
        .as_bool()
    }
}

#[cfg(any(unix, target_os = "macos"))]
pub fn set_own_process_priority(priority: Priority) -> bool {
    use libc::{getpid, setpriority, PRIO_PROCESS};

    unsafe {
        0 != setpriority(
            PRIO_PROCESS,
            getpid() as u32,
            match priority {
                Priority::Idle => 20,
                Priority::BelowNormal => 10,
                Priority::Normal => 0,
                Priority::AboveNormal => -5,
                Priority::High => -10,
                Priority::Realtime => -20,
            },
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SysInfo {
    /// Total system memory in bytes
    pub total_memory: u64,
}

#[cfg(not(any(windows, unix, target_os = "macos")))]
pub fn get_sysinfo() -> Option<SysInfo> {
    None
}

#[cfg(windows)]
pub fn get_sysinfo() -> Option<SysInfo> {
    use std::mem::MaybeUninit;

    use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};

    unsafe {
        let mut mse: MaybeUninit<MEMORYSTATUSEX> = MaybeUninit::zeroed();

        {
            // must write dwLength before call
            let length: *const u32 = memoffset::raw_field!(mse.as_mut_ptr(), MEMORYSTATUSEX, dwLength);
            (length as *mut u32).write(std::mem::size_of::<MEMORYSTATUSEX>() as u32);
        }

        if GlobalMemoryStatusEx(mse.as_mut_ptr()).as_bool() {
            let mse = mse.assume_init();
            Some(SysInfo {
                total_memory: mse.ullTotalPhys,
            })
        } else {
            None
        }
    }
}

#[cfg(any(unix, target_os = "macos"))]
pub fn get_sysinfo() -> Option<SysInfo> {
    use std::mem::MaybeUninit;

    unsafe {
        let mut si = MaybeUninit::zeroed();

        if 0 == libc::sysinfo(si.as_mut_ptr()) {
            let si = si.assume_init();

            Some(SysInfo {
                // convert to bytes with mem_unit
                total_memory: si.totalram * si.mem_unit as u64,
            })
        } else {
            None
        }
    }
}
