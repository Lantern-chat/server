#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Priority {
    Idle,
    BelowNormal,
    Normal,
    AboveNormal,
    High,
    Realtime,
}

#[cfg(not(any(target_os = "windows", target_os = "unix", target_os = "macos")))]
pub fn set_own_process_priority(priority: Priority) -> bool {
    false
}

#[cfg(target_os = "windows")]
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

#[cfg(any(target_os = "unix", target_os = "macos"))]
pub fn set_own_process_priority(priority: Priority) -> bool {
    use libc::{getpid, setpriority, PRIO_PROCESS};

    unsafe {
        0 != setpriority(
            PRIO_PROCESS,
            getpid(),
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
