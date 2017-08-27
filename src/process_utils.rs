use errors::{Error, KillError};
use libc::{self, pid_t, c_int};
use procinfo;
use procinfo::pid::Environ;
use std::ffi::OsStr;
use std::fs::read_dir;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

/// Information on running program.
#[derive(Debug, Clone)]
pub struct ProcessInfo {
    /// Process ID.
    pid: pid_t,
    /// 'Resident Set Size' in bytes.
    rss: usize,
    /// Environment variables.
    env: Environ,
    /// The time the process started after system boot.
    start_time: u64,
}

impl ProcessInfo {
    /// Returns 'Resident Set Size' in bytes.
    pub fn get_memory(&self) -> usize {
        self.rss
    }
}

/// Retrieves the number of bytes in a memory page (man 2 mmap).
/// \note Since the size can't change during the runtime it is retrieved only once and then the
///       cached value is returned.
fn get_page_size() -> usize {
    lazy_static! {
        static ref PAGE_SIZE: usize = match unsafe { libc::sysconf(libc::_SC_PAGESIZE) } {
            x if x > 0 => x as usize,
            x => panic!["Non-positive page size {}", x],
        };
    }
    *PAGE_SIZE
}

/// Sends a signal to a process.
pub fn send_signal(pid: pid_t, signal: c_int) -> Result<(), KillError> {
    trace!["Sending signal {} to process {}", signal, pid];
    match unsafe { libc::kill(pid, signal) } {
        0 => Ok(()),
        -1 => {
            let errno: c_int = unsafe { *libc::__errno_location() };
            match errno {
                libc::EINVAL => Err(KillError::InvalidSignal),
                libc::EPERM => Err(KillError::PermissionDenied),
                libc::ESRCH => Err(KillError::NotFound),
                x => panic!["Unexpected error value {}", x],
            }
        }
        x => panic!["Unexpected return code {}", x],
    }
}

/// Finds running processes with a given command name.
pub fn find_processes(cmd_name: &str) -> Result<Vec<ProcessInfo>, Error> {
    let mut results = Vec::new();
    for path in read_dir("/proc/")? {
        let path = path?;
        if !path.file_type()?.is_dir() {
            continue;
        }
        let pid = match path.file_name().into_string()?.parse::<pid_t>() {
            Ok(x) => x,
            Err(_) => continue,
        };
        let stat = procinfo::pid::stat(pid)?;
        if cmd_name != &stat.command {
            continue;
        }
        trace!["Found pid #{}", pid];
        results.push(ProcessInfo {
            pid: pid,
            rss: stat.rss * get_page_size(),
            env: procinfo::pid::environ(pid)?,
            start_time: procinfo::pid::stat(pid)?.start_time,
        });
    }
    Ok(results)
}

/// Checks whether a given process has stopped.
fn has_stopped(prog_info: &ProcessInfo) -> Result<bool, Error> {
    match send_signal(prog_info.pid, 0) {
        Err(KillError::NotFound) => {
            trace!["Process not found"];
            return Ok(true);
        }
        Err(e) => Err(e)?,
        Ok(_) => {}
    };
    trace!["Process found. Let's check if its `start_time` is the same"];
    Ok(
        procinfo::pid::stat(prog_info.pid)?.start_time != prog_info.start_time,
    )
}

/// Waits for a process to stop.
pub fn wait_stop(prog_info: &ProcessInfo, timeout: Duration) -> Result<(), Error> {
    lazy_static! {
        static ref INTERVAL: Duration = Duration::from_secs(1);
    }
    trace!["Waiting for the pid #{} to stop.", prog_info.pid];
    let started = Instant::now();
    loop {
        if has_stopped(prog_info)? {
            trace!["Process has stopped."];
            break;
        }
        if started.elapsed() > timeout {
            trace!["Timeout has been reached, leaving the process as it is."];
            break;
        }
        thread::sleep(*INTERVAL);
    }
    Ok(())
}

/// Launches and detaches a process.
pub fn launch_process<I, S>(cmd: &str, args: I, original: ProcessInfo) -> Result<(), Error>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    trace!["Launching '{}'", cmd];
    Command::new(cmd)
        .args(args)
        .env_clear()
        .envs(original.env.map(|x| match x {
            Ok(x) => x,
            Err(e) => panic!["Can't parse environment: {:?}", e],
        }))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    Ok(())
}

pub fn restart_process<I, S>(
    process: ProcessInfo,
    wait_timeout: Duration,
    cmd: &str,
    args: I,
) -> Result<(), Error>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    send_signal(process.pid, libc::SIGTERM)?;
    wait_stop(&process, wait_timeout)?;
    launch_process(cmd, args, process)?;
    Ok(())
}
