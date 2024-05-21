#![cfg(unix)]

use std::process::Child;
use std::time::Duration;

use anyhow::anyhow;
use anyhow::Error;
use gourd_lib::measurement::RUsage;
use libc::WEXITSTATUS;
use libc::WIFEXITED;

/// Returns an empty `libc::rusage` struct.
unsafe fn empty_raw_rusage() -> libc::rusage {
    std::mem::zeroed()
}

impl GetRUsage for Child {
    fn wait_for_rusage(&self) -> Result<RUsage, Error> {
        let pid = self.id() as i32;
        let mut status: i32 = 0;

        let mut rusage;
        unsafe {
            rusage = empty_raw_rusage();
            libc::wait4(
                pid,
                &mut status as *mut libc::c_int,
                0i32,
                &mut rusage as *mut libc::rusage,
            );
        }

        if WIFEXITED(status) {
            Ok(RUsage {
                utime: duration_from_timeval(rusage.ru_utime),
                stime: duration_from_timeval(rusage.ru_stime),
                maxrss: rusage.ru_maxrss as usize,
                ixrss: rusage.ru_ixrss as usize,
                idrss: rusage.ru_idrss as usize,
                isrss: rusage.ru_isrss as usize,
                minflt: rusage.ru_minflt as usize,
                majflt: rusage.ru_majflt as usize,
                nswap: rusage.ru_nswap as usize,
                inblock: rusage.ru_inblock as usize,
                oublock: rusage.ru_oublock as usize,
                msgsnd: rusage.ru_msgsnd as usize,
                msgrcv: rusage.ru_msgrcv as usize,
                nsignals: rusage.ru_nsignals as usize,
                nvcsw: rusage.ru_nvcsw as usize,
                nivcsw: rusage.ru_nivcsw as usize,
                exit_code: WEXITSTATUS(status),
            })
        } else {
            Err(anyhow!("Could not get the RUsage statistics."))
        }
    }
}

/// A trait for getting resource usage statistics for a process.
pub trait GetRUsage {
    /// Waits for the process to exit and returns its resource usage statistics.
    /// Works only on linux with wait4 syscall available.
    fn wait_for_rusage(&self) -> Result<RUsage, Error>;
}

/// Converts a `libc::timeval` to a `std::time::Duration`.
fn duration_from_timeval(timeval: libc::timeval) -> Duration {
    Duration::new(timeval.tv_sec as u64, (timeval.tv_usec * 1000) as u32)
}
